#![feature(box_patterns)]

extern crate proc_macro;

mod ref_nodes;

use lazy_static::lazy_static;
use mutable_hashset::ordered_set::MutOrderedSet as Set;
use proc_macro::TokenStream;
use proc_macro2::{Group, Span, TokenTree};
use quote::quote;
use ref_nodes::{get_ref_nodes, LifetimeOrigin, RefNode, SymbolGenerator};
use regex::Regex;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver as RX, Sender as TX};
use std::sync::Mutex;
use syn::*;

lazy_static! {
    static ref REF_DIGRAPH_CHANNELS: Mutex<HashMap<String, (TX<Vec<(String, u8)>>, RX<Vec<(String, u8)>>)>> =
        Mutex::new(HashMap::new());
}

#[proc_macro_attribute]
pub fn lifetime(args: TokenStream, input: TokenStream) -> TokenStream {
    //println!("args: {:#?}", args);
    //println!("input: {:#?}", input);

    match parse_macro_input!(input as Item) {
        Item::Struct(structure) => macro_struct(structure),
        Item::Impl(implementation) => macro_impl(implementation),
        Item::Fn(function) => {
            //println!("sig.input: {}: {:#?}", function.sig.ident, function.sig.inputs.first());
            if input_is_self(function.sig.inputs.first()) {
                macro_instance_fn(function)
            } else {
                let args = parse_macro_input!(args as AttributeArgs);
                let args: Vec<String> = args
                    .iter()
                    .filter_map(|arg| {
                        if let NestedMeta::Lit(Lit::Str(arg)) = arg {
                            Some(arg.value())
                        } else {
                            None
                        }
                    })
                    .collect();

                macro_static_fn(args, function)
            }
        }
        _ => unreachable!(""),
    }
}

fn macro_struct(mut structure: ItemStruct) -> TokenStream {
    //println!("{:#?}", structure);
    let name = structure.ident.to_string();
    let mut ref_nodes = get_ref_nodes(
        vec![LifetimeOrigin::StructFields(&mut structure.fields)],
        &mut SymbolGenerator::new(String::from("s_")),
    );

    // set generics lifetime
    for ref_node in ref_nodes.iter_mut() {
        let symbol = format!("'{}", ref_node.lf.ident);
        let lt = LifetimeDef::new(Lifetime::new(&symbol, Span::call_site()));
        structure.generics.params.push(GenericParam::from(lt));
    }

    send_ref_coords(name, ref_nodes);

    quote!(#structure).into()
}

fn macro_impl(mut implementation: ItemImpl) -> TokenStream {
    //println!("{:#?}", implementation.items);
    let symbol_generator = &mut SymbolGenerator::new(String::from("i_"));

    let mut ref_coords = vec![];
    let mut gps = HashMap::new();
    let mut edges = vec![];

    match implementation.self_ty {
        box Type::Path(TypePath {
            path: Path {
                ref mut segments, ..
            },
            ..
        }) => {
            let segment: &mut PathSegment = segments.first_mut().unwrap();
            match segment.arguments {
                PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    ref mut args,
                    ..
                }) => {
                    let name = segment.ident.to_string();

                    let coords = revc_ref_coords(name);

                    let symbols = symbol_generator.generate_n(coords.len() as u8);
                    for symbol in symbols {
                        // implementation generics lifetime
                        let lt = LifetimeDef::new(Lifetime::new(&symbol, Span::call_site()));
                        implementation.generics.params.push(GenericParam::from(lt));

                        // structure generics lifetime
                        let lt = Lifetime::new(&symbol, Span::call_site());
                        args.push(GenericArgument::Lifetime(lt));
                    }

                    ref_coords.extend(coords);
                }
                _ => (),
            }
        }
        _ => (),
    }

    for item in implementation.items.iter_mut() {
        //println!("item: {:#?}", item);
        match item {
            ImplItem::Method(iim) => {
                if input_is_self(iim.sig.inputs.first()) {
                    let fn_name = iim.sig.ident.to_string();

                    for attr in iim.attrs.iter() {
                        //println!("attr: {:#?}", attr);
                        if attr.path.segments[0].ident.to_string() == "lifetime" {
                            //println!("attr.tokens: {:#?}", attr.tokens);
                            let group: Group = syn::parse2(attr.tokens.clone()).unwrap();
                            for token in group.stream() {
                                if let TokenTree::Literal(li) = token {
                                    edges.extend(
                                        get_edges(li.to_string().trim_matches('"').to_string())
                                            .into_iter()
                                            .map(|edge| {
                                                (
                                                    if edge.0.starts_with("self.") {
                                                        edge.0
                                                    } else {
                                                        format!("{}/{}", fn_name, edge.0)
                                                    },
                                                    edge.1,
                                                    if edge.2.starts_with("self.") {
                                                        edge.2
                                                    } else {
                                                        format!("{}/{}", fn_name, edge.2)
                                                    },
                                                    edge.3,
                                                )
                                            }),
                                    );
                                }
                            }
                        }
                    }

                    let ref_nodes = get_ref_nodes(
                        vec![
                            LifetimeOrigin::FnInputs(&mut iim.sig.inputs),
                            LifetimeOrigin::FnOutout(&mut iim.sig.output),
                        ],
                        symbol_generator,
                    );

                    for ref_node in ref_nodes {
                        let lt = LifetimeDef::new(Lifetime::new(
                            &format!("'{}", &ref_node.lf.ident),
                            Span::call_site(),
                        ));
                        implementation.generics.params.push(GenericParam::from(lt));

                        ref_coords.push((format!("{}/{}", fn_name, ref_node.name), ref_node.index));
                    }
                }
            }
            _ => (),
        }
    }

    for (ref_coord, gp) in
        ref_coords
            .into_iter()
            .zip(
                implementation
                    .generics
                    .params
                    .iter_mut()
                    .filter(|gp| match gp {
                        GenericParam::Lifetime(_) => true,
                        _ => false,
                    }),
            )
    {
        gps.insert(ref_coord, gp);
    }

    println!("impl set gp ----------------------------------");
    set_gp_bounds(gps, edges);

    quote!(#implementation).into()
}

fn macro_instance_fn(mut function: ItemFn) -> TokenStream {
    quote!(#function).into()
}

fn macro_static_fn(args: Vec<String>, mut function: ItemFn) -> TokenStream {
    let mut symbol_generator = SymbolGenerator::new(String::from("f_"));
    let mut ref_nodes = get_ref_nodes(
        vec![
            LifetimeOrigin::FnInputs(&mut function.sig.inputs),
            LifetimeOrigin::FnOutout(&mut function.sig.output),
        ],
        &mut symbol_generator,
    );

    let symbols = symbol_generator.generate_n(ref_nodes.len() as u8);

    for (node, symbol) in ref_nodes.iter_mut().zip(symbols) {
        // generics lifetime
        let lt = LifetimeDef::new(Lifetime::new(&symbol, Span::call_site()));
        function.sig.generics.params.push(GenericParam::from(lt));

        // fields lifetime
        *node.lf = Lifetime::new(&symbol, Span::call_site());
    }

    let mut gps = HashMap::new();
    for (node, gp) in
        ref_nodes.iter().zip(
            function
                .sig
                .generics
                .params
                .iter_mut()
                .filter(|gp| match gp {
                    GenericParam::Lifetime(_) => true,
                    _ => false,
                }),
        )
    {
        gps.insert((node.name.clone(), node.index), gp);
    }
    let edges = args.into_iter().fold(vec![], |mut r, arg| {
        r.extend(get_edges(arg));
        r
    });

    println!(
        "fn({}) set gp ----------------------------------",
        function.sig.ident
    );
    set_gp_bounds(gps, edges);

    //println!("ref_nodes: {:#?}", ref_nodes);

    quote!(#function).into()
}

fn input_is_self(input: Option<&FnArg>) -> bool {
    //println!("input: {:#?}", input);
    match input {
        Some(FnArg::Receiver(_)) => true,
        Some(FnArg::Typed(PatType {
            pat: box Pat::Ident(pi),
            ..
        })) => {
            if pi.ident == "self" {
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

fn send_ref_coords(name: String, ref_nodes: Set<RefNode>) {
    let mut ref_nodes_channels = REF_DIGRAPH_CHANNELS.lock().unwrap();
    ref_nodes_channels.entry(name.clone()).or_insert(channel());
    let tx = &ref_nodes_channels.get(&name.clone()).unwrap().0;
    let ref_coords: Vec<(String, u8)> = ref_nodes
        .iter()
        .map(|node| (node.name.clone(), node.index as u8))
        .collect();
    let _ = tx.send(ref_coords);
}

fn revc_ref_coords(name: String) -> Vec<(String, u8)> {
    let mut ref_nodes_channels = REF_DIGRAPH_CHANNELS.lock().unwrap();
    ref_nodes_channels.entry(name.clone()).or_insert(channel());
    let rx = &ref_nodes_channels.get(&name.clone()).unwrap().1;
    let ref_coords = rx.recv().unwrap();

    //println!("ref_coords: {:?}", ref_coords);

    ref_coords
}

fn get_edges(edges: String) -> Vec<(String, u8, String, u8)> {
    let edges: String = edges.split_whitespace().collect();
    let coord_groups: Vec<&str> = edges.split("->").collect();
    let re = Regex::new(r"((?:self\.)?[a-zA-Z_][a-zA-Z0-9_]*|self)\(((?:[1-9]\d*|0)(?:,(?:[1-9]\d*|0))*)\)|((?:self\.)?[a-zA-Z_][a-zA-Z0-9_]*|self)|\(((?:[1-9]\d*|0)(?:,(?:[1-9]\d*|0))*)\)").unwrap();

    let coord_groups: Vec<Vec<(String, u8)>> = coord_groups
        .iter()
        .map(|&coord_group| {
            re.captures_iter(coord_group).fold(vec![], |mut r, caps| {
                //println!("caps: {:?}", caps);
                let name = caps
                    .get(1)
                    .or(caps.get(3))
                    .map_or("Output!", |cap| cap.as_str());
                //println!("name: {}", name);

                let indexs: Vec<u8> = caps.get(2).or(caps.get(4)).map_or(vec![0], |cap| {
                    cap.as_str()
                        .split(",")
                        .map(|i| i.parse().unwrap())
                        .collect()
                });
                //println!("indexs: {:?}", indexs);

                r.extend(
                    indexs
                        .into_iter()
                        .map(move |index| (name.to_string(), index.clone())),
                );
                r
            })
        })
        .collect();

    let mut edges = vec![];

    for (i, coord_group) in (&coord_groups[1..]).iter().enumerate() {
        for coord_b in coord_group {
            for coord_a in &coord_groups[i] {
                edges.push((coord_a.0.clone(), coord_a.1, coord_b.0.clone(), coord_b.1));
            }
        }
    }

    //println!("edges: {:?}", edges);
    //println!("\n");

    edges
}

fn set_gp_bounds(
    mut gps: HashMap<(String, u8), &mut GenericParam>,
    edges: Vec<(String, u8, String, u8)>,
) {
    println!("gps: {:?}", gps.keys());
    println!("edges: {:?}", edges);

    for edge in edges {
        println!("\nedge: {:?}", edge);
        let gp_b = gps.get(&(edge.2, edge.3)).unwrap();
        match gp_b {
            GenericParam::Lifetime(ref lf_def_b) => {
                println!("lf_def_b: {:?}", lf_def_b);
                let symbol = format!("'{}", lf_def_b.lifetime.ident);

                let gp_a = gps.get_mut(&(edge.0, edge.1)).unwrap();
                match gp_a {
                    GenericParam::Lifetime(ref mut lf_def_a) => {
                        println!("lf_def_a: {:?}", lf_def_a);
                        let lf = Lifetime::new(&symbol, Span::call_site());
                        lf_def_a.bounds.push(lf);
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
}

/*
fn get_ref_nodes(args: Vec<String>) -> RefDigraph {
    let mut ref_nodes: RefDigraph = Set::new();

    for arg in args {
        let arg: String = arg.split_whitespace().collect();
        let arg: Vec<&str> = arg.split("->").collect();

        let mut nodes_v = vec![Set::new()];

        for trs in arg {
            let mut nodes = Set::new();

            let re = Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)\(((?:[1-9]\d*|0)(?:,(?:[1-9]\d*|0))*)\)|\(((?:[1-9]\d*|0)(?:,(?:[1-9]\d*|0))*)\)").unwrap();
            for cap in re.captures_iter(trs) {
                let name = cap.get(1).map_or("Output!", |v| v.as_str());
                let indexs: Vec<u8> = cap
                    .get(2)
                    .unwrap_or_else(|| cap.get(3).unwrap())
                    .as_str()
                    .split(",")
                    .map(|index| index.parse().unwrap())
                    .collect();

                for index in indexs {
                    nodes.insert((name.to_string(), index));
                }
            }

            nodes_v.push(nodes);
        }

        for (i, nodes) in nodes_v[1..].iter().enumerate() {
            for node1 in &nodes_v[i] {
                let set = ref_nodes.entry(node1.clone()).or_default();

                for node2 in nodes {
                    set.insert(node2.clone());
                }
            }
        }
    }

    ref_nodes
}
*/

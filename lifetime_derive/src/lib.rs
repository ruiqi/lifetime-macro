#![feature(box_patterns)]

extern crate proc_macro;

mod ref_nodes;

use lazy_static::lazy_static;
use mutable_hashset::ordered_set::MutOrderedSet as Set;
use proc_macro::TokenStream;
use proc_macro2::{Group, Span, TokenTree};
use quote::quote;
use ref_nodes::{
    get_lifetime_coords, get_ref_nodes, set_lifetime_coords, LifetimeOrigin, RefNode,
    SymbolGenerator,
};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use syn::*;

lazy_static! {
    static ref LIFETIME_COORDS_MAP: Mutex<HashMap<String, Vec<(String, u8)>>> =
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
        _ => unreachable!(),
        /*
        Item::Const(_) => {}
        Item::Enum(_) => {}
        Item::ExternCrate(_) => {}
        Item::ForeignMod(_) => {}
        Item::Macro(_) => {}
        Item::Macro2(_) => {}
        Item::Mod(_) => {}
        Item::Static(_) => {}
        Item::Trait(_) => {}
        Item::TraitAlias(_) => {}
        Item::Type(_) => {}
        Item::Union(_) => {}
        Item::Use(_) => {}
        Item::Verbatim(_) => {}
        Item::__Nonexhaustive => {}
        */
    }
}

#[proc_macro_attribute]
pub fn lifetime_nothing(_: TokenStream, input: TokenStream) -> TokenStream {
    input
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

    set_lifetime_coords(name, ref_nodes);

    quote!(#structure).into()
}

fn macro_impl(mut implementation: ItemImpl) -> TokenStream {
    //println!("{:#?}", implementation);
    let symbol_generator = &mut SymbolGenerator::new(String::from("i_"));

    let mut ref_coords = vec![];
    let mut gps = HashMap::new();
    let mut edges = vec![];

    //println!("self_ty: {:#?}", implementation.self_ty);
    match implementation.self_ty {
        box Type::Path(TypePath {
            path: Path {
                ref mut segments, ..
            },
            ..
        }) => {
            for segment in segments.iter_mut() {
                let name = segment.ident.to_string();
                let coords = get_lifetime_coords(name);

                if let Some(coords) = coords {
                    let symbols = symbol_generator.generate_n(coords.len() as u8);
                    for symbol in symbols {
                        // implementation generics lifetime
                        let lt = LifetimeDef::new(Lifetime::new(&symbol, Span::call_site()));
                        implementation.generics.params.push(GenericParam::from(lt));

                        // structure generics lifetime
                        if let PathArguments::None = segment.arguments {
                            segment.arguments =
                                PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                                    colon2_token: None,
                                    lt_token: token::Lt {
                                        spans: [Span::call_site(); 1],
                                    },
                                    args: punctuated::Punctuated::new(),
                                    gt_token: token::Gt {
                                        spans: [Span::call_site(); 1],
                                    },
                                })
                        }

                        if let PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                            ref mut args,
                            ..
                        }) = segment.arguments
                        {
                            let lt = Lifetime::new(&symbol, Span::call_site());
                            args.push(GenericArgument::Lifetime(lt));
                        }
                    }

                    ref_coords.extend(coords);
                }
            }
        }
        _ => (),
    }

    for item in implementation.items.iter_mut() {
        //println!("item: {:#?}", item);
        match item {
            ImplItem::Method(iim) => {
                let fn_name = iim.sig.ident.to_string();

                for attr in iim.attrs.iter_mut() {
                    //println!("attr: {:#?}", attr);
                    if attr.path.segments[0].ident.to_string() == "lifetime" {
                        attr.path.segments[0].ident =
                            Ident::new("lifetime_nothing", Span::call_site());

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
            _ => unreachable!(),
            /*
            ImplItem::Const(_) => {}
            ImplItem::Type(_) => {}
            ImplItem::Macro(_) => {}
            ImplItem::Verbatim(_) => {}
            ImplItem::__Nonexhaustive => {}
            */
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

    set_gp_bounds(gps, edges);

    //println!("ref_nodes: {:#?}", ref_nodes);

    quote!(#function).into()
}

fn get_edges(edges: String) -> Vec<(String, u8, String, u8)> {
    let edges: String = edges.split_whitespace().collect();
    let coord_groups: Vec<&str> = edges.split("->").collect();
    let re = Regex::new(r"((?:[a-zA-Z_][a-zA-Z0-9_]*\.)*[a-zA-Z_][a-zA-Z0-9_]*|self)\(((?:[1-9]\d*|0)(?:,(?:[1-9]\d*|0))*)\)|((?:[a-zA-Z_][a-zA-Z0-9_]*\.)*[a-zA-Z_][a-zA-Z0-9_]*|self)|\(((?:[1-9]\d*|0)(?:,(?:[1-9]\d*|0))*)\)").unwrap();

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
    println!("gps keys: {:?}", gps.keys());
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

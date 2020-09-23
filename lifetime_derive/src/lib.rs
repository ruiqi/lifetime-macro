#![feature(box_patterns)]

extern crate proc_macro;

mod ref_nodes;

use alias_trie::Trie;
use itertools::concat;
use lazy_static::lazy_static;
use proc_macro::TokenStream;
use proc_macro2::{Group, Span, TokenTree};
use quote::quote;
use ref_nodes::{get_ref_digrphs, RDigrph, RNode, ROrigin};
use regex::{Captures, Regex};
use std::collections::HashMap;
use std::sync::Mutex;
use syn::*;

lazy_static! {
    static ref LIFETIME_COORDS_MAP: Mutex<HashMap<String, Vec<(String, u8)>>> =
        Mutex::new(HashMap::new());
}

#[proc_macro_attribute]
pub fn lifetime(args: TokenStream, input: TokenStream) -> TokenStream {
    match parse_macro_input!(input as Item) {
        Item::Struct(struct_) => macro_struct(struct_),
        Item::Enum(enum_) => macro_enum(enum_),
        Item::Impl(impl_) => macro_impl(impl_),
        Item::Fn(fn_) => {
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

            macro_fn(args, fn_)
        }
        Item::Trait(trait_) => macro_trait(trait_),
        _ => unreachable!(),
        /*
        Item::Const(_) => {}
        Item::ExternCrate(_) => {}
        Item::ForeignMod(_) => {}
        Item::Macro(_) => {}
        Item::Macro2(_) => {}
        Item::Mod(_) => {}
        Item::Static(_) => {}
        Item::TraitAlias(_) => {}
        Item::Type(_) => {}
        Item::Union(_) => {}
        Item::Use(_) => {}
        Item::Verbatim(_) => {}
        Item::__Nonexhaustive => {}
        */
    }
}

fn macro_struct(mut struct_: ItemStruct) -> TokenStream {
    //println!("{:#?}", struct_);
    let symbol_generator = &mut SymbolGenerator::new(String::from("s_"));

    let name = struct_.ident.to_string();
    let origins = vec![ROrigin::StructFields(&mut struct_.fields)];
    let mut digrphs = get_ref_digrphs(name.clone(), origins);

    set_lifetime_symbols(Some(&mut struct_.generics), &mut digrphs, symbol_generator);
    set_lifetime_coords(name, &mut digrphs);

    quote!(#struct_).into()
}

fn macro_enum(mut enum_: ItemEnum) -> TokenStream {
    //println!("{:#?}", enum_);
    let symbol_generator = &mut SymbolGenerator::new(String::from("e_"));

    let name = enum_.ident.to_string();
    let origins = vec![ROrigin::EnumVariants(&mut enum_.variants)];
    let mut digrphs = get_ref_digrphs(name.clone(), origins);

    set_lifetime_symbols(Some(&mut enum_.generics), &mut digrphs, symbol_generator);
    set_lifetime_coords(name, &mut digrphs);

    quote!(#enum_).into()
}

fn macro_impl(mut impl_: ItemImpl) -> TokenStream {
    //println!("{:#?}", impl_);

    //let local_lifetime_coords_map = HashMap::new();
    let symbol_generator = &mut SymbolGenerator::new(String::from("i_"));

    let mut coords = vec![];

    // impl_.self_ty
    //println!("impl_.self_ty: {:#?}", impl_.self_ty);
    let origins = vec![ROrigin::SelfTY(&mut impl_.self_ty)];
    let mut digrphs = get_ref_digrphs("".to_string(), origins);
    // set lifetime symbols
    set_lifetime_symbols(Some(&mut impl_.generics), &mut digrphs, symbol_generator);
    // coords
    coords.extend(concat(digrphs.iter().map(|digrph| digrph.get_coords())));

    // impl_.generics
    let origins = vec![ROrigin::Generics(&mut impl_.generics)];
    let mut digrphs = get_ref_digrphs("generics".to_string(), origins);
    // set lifetime symbols
    set_lifetime_symbols(Some(&mut impl_.generics), &mut digrphs, symbol_generator);
    // coords
    coords.extend(concat(digrphs.iter().map(|digrph| digrph.get_coords())));

    // impl_.trait_
    let origins = vec![ROrigin::Trait(&mut impl_.trait_)];
    let mut digrphs = get_ref_digrphs("trait_".to_string(), origins);
    // set lifetime symbols
    set_lifetime_symbols(None, &mut digrphs, symbol_generator);

    // `impl_.items` immediately follow `impl_.trait_`, because their symbols is the same.
    // The `symbol_generator` will roll back the symbols of the `impl_.trait_`.
    // impl_.items
    for item in impl_.items.iter_mut() {
        //println!("item: {:#?}", item);
        match item {
            ImplItem::Method(iim) => {
                let name = iim.sig.ident.to_string();

                // set lifetime symbols
                let origins = vec![
                    ROrigin::FnInputs(&mut iim.sig.inputs),
                    ROrigin::FnOutput(&mut iim.sig.output),
                ];
                let mut digrphs = get_ref_digrphs(name, origins);
                set_lifetime_symbols(Some(&mut impl_.generics), &mut digrphs, symbol_generator);

                // coords
                coords.extend(concat(digrphs.iter().map(|digrph| digrph.get_coords())));
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

    // edges
    let mut edges = vec![];
    for item in impl_.items.iter_mut() {
        //println!("item: {:#?}", item);
        match item {
            ImplItem::Method(iim) => {
                let name = iim.sig.ident.to_string();

                for attr in iim.attrs.iter() {
                    if attr.path.segments[0].ident.to_string() == "lifetime" {
                        let group: Group = syn::parse2(attr.tokens.clone()).unwrap();
                        for token in group.stream() {
                            if let TokenTree::Literal(li) = token {
                                edges.extend(get_edges(
                                    name.clone(),
                                    li.to_string().trim_matches('"').to_string(),
                                ));
                            }
                        }
                    }
                }

                // remove instance lifetime macro
                iim.attrs
                    .retain(|attr| attr.path.segments[0].ident.to_string() != "lifetime");
            }
            _ => (),
        }
    }

    set_generic_lifetime_bounds(
        impl_
            .generics
            .params
            .iter_mut()
            .filter(|gp| match gp {
                GenericParam::Lifetime(_) => true,
                _ => false,
            })
            .collect::<Vec<_>>(),
        coords,
        edges,
    );

    quote!(#impl_).into()
}

fn macro_fn(args: Vec<String>, mut fn_: ItemFn) -> TokenStream {
    let symbol_generator = &mut SymbolGenerator::new(String::from("f_"));

    let mut coords = vec![];

    // fn_.generics
    let origins = vec![ROrigin::Generics(&mut fn_.sig.generics)];
    let mut digrphs = get_ref_digrphs("generics".to_string(), origins);
    // set lifetime symbols
    set_lifetime_symbols(Some(&mut fn_.sig.generics), &mut digrphs, symbol_generator);
    // coords
    coords.extend(concat(digrphs.iter().map(|digrph| digrph.get_coords())));

    // fn_.sig.inputs, fn_.sig.output
    let origins = vec![
        ROrigin::FnInputs(&mut fn_.sig.inputs),
        ROrigin::FnOutput(&mut fn_.sig.output),
    ];
    let mut digrphs = get_ref_digrphs("".to_string(), origins);
    // set lifetime symbols
    set_lifetime_symbols(Some(&mut fn_.sig.generics), &mut digrphs, symbol_generator);
    // coords
    coords.extend(concat(digrphs.iter().map(|digrph| digrph.get_coords())));

    // edges
    let edges = concat(args.into_iter().map(|arg| get_edges("".to_string(), arg)));

    set_generic_lifetime_bounds(
        fn_.sig
            .generics
            .params
            .iter_mut()
            .filter(|gp| match gp {
                GenericParam::Lifetime(_) => true,
                _ => false,
            })
            .collect::<Vec<_>>(),
        coords,
        edges,
    );

    quote!(#fn_).into()
}

fn macro_trait(mut trait_: ItemTrait) -> TokenStream {
    //println!("trait_: {:#?}", trait_);

    let symbol_generator = &mut SymbolGenerator::new(String::from("t_"));

    let name = trait_.ident.to_string();
    let mut digrphs = vec![];

    for item in trait_.items.iter_mut() {
        match item {
            TraitItem::Method(tim) => {
                let fn_name = tim.sig.ident.to_string();

                let origins = vec![
                    ROrigin::FnInputs(&mut tim.sig.inputs),
                    ROrigin::FnOutput(&mut tim.sig.output),
                ];

                digrphs.extend(get_ref_digrphs(format!("{}/{}", name, fn_name), origins));
            }
            _ => (),
            /*
            TraitItem::Const(_) => {}
            TraitItem::Type(_) => {}
            TraitItem::Macro(_) => {}
            TraitItem::Verbatim(_) => {}
            TraitItem::__Nonexhaustive => {}
            */
        }
    }

    set_lifetime_symbols(Some(&mut trait_.generics), &mut digrphs, symbol_generator);
    set_lifetime_coords(name, &mut digrphs);

    quote!(#trait_).into()
}

fn get_edges(namespace: String, edges: String) -> Vec<(String, u8, String, u8)> {
    let re = Regex::new(r"([a-zA-Z_][a-zA-Z0-9_!]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*|\.(?:[1-9]\d*|0)|\[[a-zA-Z_][a-zA-Z0-9_]*(?:,(?:[1-9]\d*|0))?\])*)\(((?:[1-9]\d*|0)(?:,(?:[1-9]\d*|0))*)\)|([a-zA-Z_][a-zA-Z0-9_!]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*|\.(?:[1-9]\d*|0)|\[[a-zA-Z_][a-zA-Z0-9_]*(?:,(?:[1-9]\d*|0))?\])*)|\(((?:[1-9]\d*|0)(?:,(?:[1-9]\d*|0))*)\)").unwrap();

    let coord_groups = edges
        .split_whitespace()
        .collect::<String>()
        .split("->")
        .map(|coord_group| {
            re.captures_iter(coord_group).fold(vec![], |mut r, caps| {
                let name = caps
                    .get(1)
                    .or(caps.get(3))
                    .map_or("Output!", |cap| cap.as_str());
                let name = Regex::new(r"\[([^,\[\]]+)\]")
                    .unwrap()
                    .replace(name, |caps: &Captures| format!("[{},0]", &caps[1]));

                let indexs: Vec<u8> = caps.get(2).or(caps.get(4)).map_or(vec![0], |cap| {
                    cap.as_str()
                        .split(",")
                        .map(|i| i.parse().unwrap())
                        .collect()
                });

                // $ is end
                r.extend(
                    indexs
                        .into_iter()
                        .map(move |index| (format!("{}$", name), index.clone())),
                );
                r
            })
        })
        .collect::<Vec<Vec<(String, u8)>>>();

    let mut edges = vec![];

    for (i, coord_group) in (&coord_groups[1..]).iter().enumerate() {
        for coord_b in coord_group {
            for coord_a in &coord_groups[i] {
                edges.push((
                    if coord_a.0.starts_with("self.") {
                        coord_a.0.clone()
                    } else {
                        if namespace.is_empty() {
                            coord_a.0.clone()
                        } else {
                            format!("{}/{}", namespace, coord_a.0)
                        }
                    },
                    coord_a.1,
                    if coord_b.0.starts_with("self.") {
                        coord_b.0.clone()
                    } else {
                        if namespace.is_empty() {
                            coord_b.0.clone()
                        } else {
                            format!("{}/{}", namespace, coord_b.0)
                        }
                    },
                    coord_b.1,
                ));
            }
        }
    }

    edges
}

fn set_generic_lifetime_bounds(
    //mut generic_lifetimes_map: HashMap<(String, u8), &mut GenericParam>,
    gps: Vec<&mut GenericParam>,
    coords: Vec<(String, u8)>,
    edges: Vec<(String, u8, String, u8)>,
) {
    println!("\n======================================\n");
    println!("coords: {:?}", coords);
    println!("edges: {:?}", edges);

    let mut generic_lifetimes_map = HashMap::new();

    for (coord, gp) in coords.iter().zip(gps.into_iter().filter(|gp| match gp {
        GenericParam::Lifetime(_) => true,
        _ => false,
    })) {
        generic_lifetimes_map.insert((*coord).clone(), gp);
    }

    let abbr_names_trie = get_abbr_names_trie(coords);
    //println!("trie: {:#?}", abbr_names_trie);

    for (name1, index1, name2, index2) in edges {
        // change abbr name to full name
        //println!(
        //    "before edge: ({}, {}) -> ({}, {})",
        //    name1, index1, name2, index2
        //);
        let name1 = abbr_names_trie.get(&name1.split(".").collect::<Vec<_>>());
        //println!("name1: {:?}", name1);
        let name1 = (*name1.unwrap()).clone();
        let name2 = abbr_names_trie.get(&name2.split(".").collect::<Vec<_>>());
        //println!("name2: {:?}", name2);
        let name2 = (*name2.unwrap()).clone();
        //println!(
        //    "after edge: ({}, {}) -> ({}, {})",
        //    name1, index1, name2, index2
        //);

        let param_b = generic_lifetimes_map.get(&(name2, index2)).unwrap();
        match param_b {
            GenericParam::Lifetime(ref lf_def_b) => {
                //println!("lf_def_b: {:?}", lf_def_b);
                let symbol = format!("'{}", lf_def_b.lifetime.ident);

                let param_a = generic_lifetimes_map.get_mut(&(name1, index1)).unwrap();
                match param_a {
                    GenericParam::Lifetime(ref mut lf_def_a) => {
                        //println!("lf_def_a: {:?}", lf_def_a);
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

fn set_lifetime_coords(name: String, digrphs: &Vec<RDigrph>) {
    let mut lifetime_coords_map = LIFETIME_COORDS_MAP.lock().unwrap();

    let coords = concat(digrphs.iter().map(|digrph| digrph.get_coords()));

    //println!("[{}] set lifetime coords: {:?}", name, coords);
    lifetime_coords_map.insert(name, coords);
}

fn get_lifetime_coords(name: String) -> Vec<(String, u8)> {
    let lifetime_coords_map = LIFETIME_COORDS_MAP.lock().unwrap();
    let lifetime_coords = lifetime_coords_map
        .get(&name.clone())
        .map(|v| (*v).clone())
        .unwrap_or(vec![]);

    //println!("[{}] get lifetime coords: {:?}", name, lifetime_coords);
    lifetime_coords
}

struct SymbolGenerator {
    perfix: String,
    letter: char,
    number: u8,
    snapshot: (char, u8),
}

impl SymbolGenerator {
    fn new(perfix: String) -> Self {
        SymbolGenerator {
            perfix: perfix,
            letter: 'a',
            number: 0,
            snapshot: ('a', 0),
        }
    }

    fn generate(&mut self) -> String {
        let symbol = if self.number == 0 {
            format!("'{}{}", self.perfix, self.letter)
        } else {
            format!("'{}{}{}", self.perfix, self.letter, self.number)
        };

        if self.letter == 'z' {
            self.letter = 'a';
            self.number += 1;
        } else {
            self.letter = (self.letter as u8 + 1) as char;
        }

        symbol
    }

    fn generate_n(&mut self, n: u8) -> Vec<String> {
        (0..n).map(|_| self.generate()).collect()
    }

    fn reset(&mut self) {
        self.letter = 'a';
        self.number = 0;
    }

    fn skip(&mut self, n: usize) {
        self.generate_n(n as u8);
    }

    fn take_a_snapshot(&mut self) {
        self.snapshot = (self.letter, self.number);
    }

    fn rollback(&mut self) {
        self.letter = self.snapshot.0;
        self.number = self.snapshot.1;
    }

    fn regenerate(&mut self) -> Vec<String> {
        let mut symbols = vec![];

        let snapshot = (self.letter, self.number);
        self.rollback();

        while (self.letter, self.number) != snapshot {
            symbols.push(self.generate());
        }

        symbols
    }
}

fn set_lifetime_symbols(
    generics: Option<&mut Generics>,
    digrphs: &mut Vec<RDigrph>,
    symbol_generator: &mut SymbolGenerator,
) {
    symbol_generator.take_a_snapshot();

    for digrph in digrphs.iter_mut() {
        for node in digrph.nodes.iter_mut() {
            match node {
                RNode::Lifetime(node) => unsafe {
                    let symbol = symbol_generator.generate();

                    // refercence lifetime
                    (*node.lifetime).ident = Ident::new(&symbol[1..], Span::call_site());
                },
                RNode::Segment(node) => unsafe {
                    let name = (*node.segment).ident.to_string();
                    let coords = get_lifetime_coords(name);

                    for _ in coords.iter() {
                        let symbol = symbol_generator.generate();

                        // arguments lifetime
                        if let PathArguments::None = (*node.segment).arguments {
                            (*node.segment).arguments =
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
                        }) = (*node.segment).arguments
                        {
                            let lt = Lifetime::new(&symbol, Span::call_site());
                            args.push(GenericArgument::Lifetime(lt));
                        }
                    }

                    node.coords = Some(coords);
                },
            }
        }
    }

    // generics lifetime
    if let Some(generics) = generics {
        for symbol in symbol_generator.regenerate() {
            let lt = LifetimeDef::new(Lifetime::new(symbol.as_str(), Span::call_site()));
            generics.params.push(GenericParam::from(lt));
        }
    } else {
        symbol_generator.rollback();
    }
}

fn get_abbr_names_trie(coords: Vec<(String, u8)>) -> Trie<String, String> {
    let mut trie = Trie::new();

    let names = coords
        .iter()
        .map(|(name, _)| (*name).clone())
        .collect::<Vec<_>>();

    for name in names {
        let path = name.split(".").map(|s| s.to_string()).collect::<Vec<_>>();

        trie.insert(&path, name);

        let re = Regex::new(r"\[[^\[\]]+\]").unwrap();
        let path_aliases = path
            .iter()
            .map(|cell1| {
                let cell2 = re.replace(cell1, "").to_string();
                if cell1.len() == cell2.len() {
                    vec![]
                } else {
                    vec![cell2]
                }
            })
            .collect::<Vec<_>>();

        let path = &path.iter().map(|s| s).collect::<Vec<_>>()[..];
        let path_aliases = &path_aliases
            .iter()
            .map(|alies| &alies[..])
            .collect::<Vec<_>>()[..];
        trie.update_aliases(path, path_aliases);
    }

    trie
}

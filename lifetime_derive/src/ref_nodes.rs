use super::{LIFETIME_COORDS_MAP};
use mutable_hashset::ordered_set::MutOrderedSet as Set;
use proc_macro2::Span;
use std::hash::{Hash, Hasher};
use syn::punctuated::Punctuated;
use syn::*;

enum RDigrphORRnode<'a> {
    RDigrph(*mut RDigrph<'a>),
    RNode(RNode<'a>),
}
pub struct RNode<'a> {
    pub name: String,
    pub lf: &'a mut Lifetime,
}

struct RDigrph<'a> {
    pub name: String,
    pub nodes: Vec<RDigrphORRnode<'a>>,
}

impl<'a> RDigrph<'a> {
    fn get_coords(&self) -> Vec<(String, u8)> {
        let mut coords = vec![];

        for (i, node) in self.nodes.iter().enumerate() {
            match node {
                RDigrphORRnode::RDigrph(digrph) => unsafe {
                    coords.extend((**digrph).get_coords());
                }
                RDigrphORRnode::RNode(node) => {
                    coords.push((node.name.clone(), i as u8))
                }
            }
        }

        coords
    }
}

#[derive(Debug)]
pub struct RefNode<'a> {
    pub name: String,
    pub index: u8,
    pub lf: &'a mut Lifetime,
}

impl<'a> PartialEq for RefNode<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.index == other.index
    }
}
impl<'a> Eq for RefNode<'a> {}

impl<'a> Hash for RefNode<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.index.hash(state);
    }
}

pub enum LifetimeOrigin<'a> {
    FnInputs(&'a mut Punctuated<FnArg, Token![,]>),
    FnOutput(&'a mut ReturnType),
    StructFields(&'a mut Fields),
}

pub fn get_ref_nodes<'a>(
    origins: Vec<LifetimeOrigin<'a>>,
    symbol_generator: &mut SymbolGenerator,
) -> Set<RefNode<'a>> {
    let mut nodes = Set::new();

    for origin in origins {
        match origin {
            LifetimeOrigin::FnInputs(inputs) => {
                for input in inputs.iter_mut() {
                    match input {
                        FnArg::Receiver(Receiver {
                            reference: Some((_, olf)),
                            //self_token,
                            ..
                        }) => {
                            *olf = Some(Lifetime::new(
                                &symbol_generator.generate(),
                                Span::call_site(),
                            ));

                            nodes.insert(RefNode {
                                name: String::from("self"),
                                index: 0,
                                lf: olf.as_mut().unwrap(),
                            });
                        }
                        FnArg::Typed(pt) => {
                            println!("pt: {:#?}", pt);
                            nodes.extend(get_ref_nodes_from_type(
                                get_name_from_pat(&pt.pat),
                                &mut 0,
                                &mut *pt.ty,
                                symbol_generator,
                            ));
                        }
                        _ => (),
                    }
                }
            }
            LifetimeOrigin::FnOutput(output) => match output {
                ReturnType::Type(_, box ref mut ty) => {
                    nodes.extend(get_ref_nodes_from_type(
                        String::from("Output!"),
                        &mut 0,
                        ty,
                        symbol_generator,
                    ));
                }
                _ => (),
            },
            LifetimeOrigin::StructFields(fields) => {
                for (i, field) in fields.iter_mut().enumerate() {
                    nodes.extend(get_ref_nodes_from_type(
                        format!(
                            "self.{}",
                            field
                                .ident
                                .as_ref()
                                .map_or(i.to_string(), |ident| ident.to_string())
                        ),
                        &mut 0,
                        &mut field.ty,
                        symbol_generator,
                    ));
                }
            }
        }
    }

    nodes
}

fn get_ref_nodes_from_type<'a>(
    name: String,
    index: &mut u8,
    ty: &'a mut Type,
    symbol_generator: &mut SymbolGenerator,
) -> Set<RefNode<'a>> {
    //println!("ty: {:#?}", ty);
    let mut nodes = Set::new();

    match ty {
        Type::Reference(ref mut tr) => {
            tr.lifetime = Some(Lifetime::new(
                &symbol_generator.generate(),
                Span::call_site(),
            ));

            nodes.insert(RefNode {
                name: name.clone(),
                index: *index,
                lf: tr.lifetime.as_mut().unwrap(),
            });
            *index += 1;

            nodes.extend(get_ref_nodes_from_type(
                name.clone(),
                index,
                &mut *tr.elem,
                symbol_generator,
            ));
        }
        Type::Tuple(ref mut tt) => {
            for elem in tt.elems.iter_mut() {
                nodes.extend(get_ref_nodes_from_type(
                    name.clone(),
                    index,
                    elem,
                    symbol_generator,
                ));
            }
        }
        Type::Path(TypePath {
            ref mut qself,
            path: ref mut pt,
            ..
        }) => {
            if let Some(qself) = qself {
                nodes.extend(get_ref_nodes_from_type(
                    name.clone(),
                    index,
                    &mut *qself.ty,
                    symbol_generator,
                ));
            }

            nodes.extend(get_ref_nodes_from_path(
                name.clone(),
                index,
                pt,
                symbol_generator,
            ));
        }
        _ => {
            //println!("ty: {:#?}", ty);
            unreachable!()
        } /*
          Type::Array(_) => {}
          Type::BareFn(_) => {}
          Type::Group(_) => {}
          Type::ImplTrait(_) => {}
          Type::Infer(_) => {}
          Type::Macro(_) => {}
          Type::Never(_) => {}
          Type::Paren(_) => {}
          Type::Ptr(_) => {}
          Type::Slice(_) => {}
          Type::TraitObject(_) => {}
          Type::Verbatim(_) => {}
          Type::__Nonexhaustive => {}
          */
    }

    nodes
}

fn get_ref_nodes_from_path<'a>(
    name: String,
    index: &mut u8,
    path: &'a mut Path,
    symbol_generator: &mut SymbolGenerator,
) -> Set<RefNode<'a>> {
    println!("name: {}", name);
    println!("path: {:#?}\nident:{:?}", path, path.get_ident());
    
    let mut nodes = Set::new();

    let segments_coords = set_path_lfs_and_get_coords(path, symbol_generator);

    for (segment, coords) in path.segments.iter_mut().zip(segments_coords) {
        match segment.arguments {
            PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                ref mut args, ..
            }) => {
                for arg in args {
                    match arg {
                        GenericArgument::Type(ref mut ty) => {
                            nodes.extend(get_ref_nodes_from_type(
                                format!("{}", name),
                                index,
                                ty,
                                symbol_generator,
                            ));
                        }
                        GenericArgument::Binding(Binding { ref mut ty, .. }) => {
                            nodes.extend(get_ref_nodes_from_type(
                                name.clone(),
                                index,
                                ty,
                                symbol_generator,
                            ));
                        }
                        GenericArgument::Constraint(Constraint { ref mut bounds, .. }) => {
                            for bound in bounds {
                                match bound {
                                    TypeParamBound::Trait(TraitBound {
                                        path: ref mut pt, ..
                                    }) => {
                                        nodes.extend(get_ref_nodes_from_path(
                                            name.clone(),
                                            index,
                                            pt,
                                            symbol_generator,
                                        ));
                                    }
                                    _ => (),
                                }
                            }
                        }
                        GenericArgument::Lifetime(lf) => {
                            nodes.insert(RefNode {
                                name: name.clone(),
                                index: *index,
                                lf: lf,
                            });
                            *index += 1;
                        }
                        _ => (), /*
                                 GenericArgument::Const(_) => {}
                                 */
                    }
                }
            }
            PathArguments::Parenthesized(ParenthesizedGenericArguments {
                ref mut inputs,
                ref mut output,
                ..
            }) => {
                for input in inputs {
                    nodes.extend(get_ref_nodes_from_type(
                        name.clone(),
                        index,
                        input,
                        symbol_generator,
                    ));
                }

                if let ReturnType::Type(_, box ref mut ty) = output {
                    nodes.extend(get_ref_nodes_from_type(
                        name.clone(),
                        index,
                        ty,
                        symbol_generator,
                    ));
                }
            }
            PathArguments::None => {}
        }
    }

    nodes
}

fn set_path_lfs_and_get_coords(pt: &mut Path, symbol_generator: &mut SymbolGenerator) -> Vec<Option<Vec<(String, u8)>>>{
    let mut segments_coords = vec![];

    for segment in pt.segments.iter_mut() {
        let name = segment.ident.to_string();
        let coords = get_lifetime_coords(name.clone());

        if let Some(coords) = coords.clone() {
            let symbols = symbol_generator.generate_n(coords.len() as u8);

            for symbol in symbols {
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
        }

        segments_coords.push(coords);
    }

    segments_coords
}

fn get_name_from_pat(pat: &Pat) -> String {
    match pat {
        Pat::Box(PatBox { box ref pat, .. }) => get_name_from_pat(pat),
        Pat::Ident(pi) => pi.ident.to_string(),
        Pat::Reference(PatReference { box ref pat, .. }) => get_name_from_pat(pat),
        Pat::Type(PatType { box ref pat, .. }) => get_name_from_pat(pat),
        _ => unreachable!(),
    }
}

pub fn set_lifetime_coords(name: String, ref_nodes: Set<RefNode>) {
    let mut lifetime_coords_map = LIFETIME_COORDS_MAP.lock().unwrap();
    //let coords = lifetime_coords_map.get(&name.clone()).unwrap();
    let lifetime_coords: Vec<(String, u8)> = ref_nodes
        .iter()
        .map(|node| (node.name.clone(), node.index as u8))
        .collect();

    println!("[{}] set lifetime coords: {:?}", name, lifetime_coords);
    lifetime_coords_map.insert(name.clone(), lifetime_coords);
}

pub fn get_lifetime_coords(name: String) -> Option<Vec<(String, u8)>> {
    let lifetime_coords_map = LIFETIME_COORDS_MAP.lock().unwrap();
    let lifetime_coords = lifetime_coords_map.get(&name.clone()).map(|v| (*v).clone());

    println!("[{}] get lifetime coords: {:?}", name, lifetime_coords);
    lifetime_coords
}

pub struct SymbolGenerator {
    perfix: String,
    letter: char,
    number: u8,
}

impl SymbolGenerator {
    pub fn new(perfix: String) -> Self {
        SymbolGenerator {
            perfix: perfix,
            letter: 'a',
            number: 0,
        }
    }

    pub fn generate(&mut self) -> String {
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

    pub fn generate_n(&mut self, n: u8) -> Vec<String> {
        (0..n).map(|_| self.generate()).collect()
    }
}

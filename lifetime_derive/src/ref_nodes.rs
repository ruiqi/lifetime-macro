use mutable_hashset::ordered_set::MutOrderedSet as Set;
use proc_macro2::Span;
use std::hash::{Hash, Hasher};
use syn::punctuated::Punctuated;
use syn::*;

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
    FnOutout(&'a mut ReturnType),
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
            LifetimeOrigin::FnOutout(output) => match output {
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
            println!("ty: {:#?}", ty);
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
    pt: &'a mut Path,
    symbol_generator: &mut SymbolGenerator,
) -> Set<RefNode<'a>> {
    let mut nodes = Set::new();

    for segment in pt.segments.iter_mut() {
        match segment.arguments {
            PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                ref mut args, ..
            }) => {
                for arg in args {
                    match arg {
                        GenericArgument::Type(ref mut ty) => {
                            nodes.extend(get_ref_nodes_from_type(
                                name.clone(),
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
                        _ => (),
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

fn get_name_from_pat(pat: &Pat) -> String {
    match pat {
        Pat::Box(PatBox { box ref pat, .. }) => get_name_from_pat(pat),
        Pat::Ident(pi) => pi.ident.to_string(),
        Pat::Reference(PatReference { box ref pat, .. }) => get_name_from_pat(pat),
        Pat::Type(PatType { box ref pat, .. }) => get_name_from_pat(pat),
        _ => unreachable!(),
    }
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

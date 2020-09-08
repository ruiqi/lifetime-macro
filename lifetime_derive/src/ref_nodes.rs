use super::LIFETIME_COORDS_MAP;
use mutable_hashset::ordered_set::MutOrderedSet as Set;
use proc_macro2::Span;
use std::hash::{Hash, Hasher};
use syn::punctuated::Punctuated;
use syn::*;

#[derive(Debug)]
pub struct LifetimeNode<'a> {
    pub lifetime: &'a mut Lifetime,
}

impl<'a> LifetimeNode<'a> {
    fn new(lifetime: &'a mut Lifetime) -> Self {
        Self { lifetime: lifetime }
    }
}

#[derive(Debug)]
pub struct SegmentNode<'a> {
    pub segment: &'a mut PathSegment,
    pub coords: Option<Vec<(String, u8)>>,
}

impl<'a> SegmentNode<'a> {
    fn new(segment: &'a mut PathSegment) -> Self {
        Self {
            segment: segment,
            coords: None,
        }
    }
}

#[derive(Debug)]
pub struct ArgumentsNode<'a> {
    pub name: String,
    pub arguments: &'a mut AngleBracketedGenericArguments,
    pub coords: Option<Vec<(String, u8)>>,
}

impl<'a> ArgumentsNode<'a> {
    fn new(name: String, arguments: &'a mut AngleBracketedGenericArguments) -> Self {
        Self {
            name: name,
            arguments: arguments,
            coords: None,
        }
    }
}

#[derive(Debug)]
pub enum RNode<'a> {
    Lifetime(LifetimeNode<'a>),
    Segment(SegmentNode<'a>),
    Arguments(ArgumentsNode<'a>),
}

impl<'a> RNode<'a> {
    fn new_lifetime(lifetime: &'a mut Lifetime) -> Self {
        Self::Lifetime(LifetimeNode::new(lifetime))
    }

    fn new_segment(segment: &'a mut PathSegment) -> Self {
        Self::Segment(SegmentNode::new(segment))
    }

    fn new_arguments(name: String, arguments: &'a mut AngleBracketedGenericArguments) -> Self {
        Self::Arguments(ArgumentsNode::new(name, arguments))
    }
}

#[derive(Debug)]
pub struct RDigrph<'a> {
    pub name: String,
    pub nodes: Vec<RNode<'a>>,
}

impl<'a> RDigrph<'a> {
    fn new(name: String) -> Self {
        Self {
            name: name,
            nodes: vec![],
        }
    }

    pub fn get_coords(&self) -> Vec<(String, u8)> {
        let mut coords = vec![];

        // lifetime coords
        let lifetime_node_count = self
            .nodes
            .iter()
            .filter(|node| {
                if let RNode::Lifetime(_) = node {
                    true
                } else {
                    false
                }
            })
            .count();
        coords.extend((0..lifetime_node_count).map(|i| (self.name.clone(), i as u8)));

        // segment coords
        for node in self.nodes.iter() {
            match node {
                RNode::Segment(SegmentNode {
                    coords: Some(cds), ..
                }) => {
                    coords.extend(
                        cds.iter()
                            .map(|cd| (format!("{}/{}", self.name, cd.0), cd.1)),
                    );
                }
                _ => (),
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

pub enum ROrigin<'a> {
    FnInputs(&'a mut Punctuated<FnArg, Token![,]>),
    FnOutput(&'a mut ReturnType),
    StructFields(&'a mut Fields),
}

pub fn get_ref_digrphs<'a>(name: String, origins: Vec<ROrigin<'a>>) -> Vec<RDigrph<'a>> {
    let mut digrphs = vec![];

    for origin in origins {
        match origin {
            ROrigin::FnInputs(inputs) => {
                for input in inputs.iter_mut() {
                    let mut digrph = RDigrph::new("null".to_string());

                    match input {
                        FnArg::Receiver(Receiver {
                            reference: Some((_, olf)),
                            ..
                        }) => {
                            *olf = Some(Lifetime::new("'null", Span::call_site()));

                            digrph.name = format!("{}/{}", name, "self");
                            digrph.nodes.push(RNode::new_lifetime(olf.as_mut().unwrap()));
                        }
                        FnArg::Typed(pt) => {
                            println!("pt: {:#?}", pt);

                            digrph.name = format!("{}/{}", name, get_name_from_pat(&pt.pat));
                            digrph.nodes.extend(get_ref_nodes_from_type(&mut *pt.ty));
                        }
                        _ => (),
                    }

                    digrphs.push(digrph);
                }
            }
            ROrigin::FnOutput(output) => {
                let mut digrph = RDigrph::new(format!("{}/{}", name, "Output!"));

                match output {
                    ReturnType::Type(_, box ref mut ty) => {
                        digrph.nodes.extend(get_ref_nodes_from_type(ty));
                    }
                    _ => (),
                }

                digrphs.push(digrph);
            }
            ROrigin::StructFields(fields) => {
                for (i, field) in fields.iter_mut().enumerate() {
                    let field_name = field
                        .ident
                        .as_ref()
                        .map_or(i.to_string(), |ident| ident.to_string());
                    let mut digrph = RDigrph::new(format!("{}/{}", name, field_name));

                    digrph.nodes.extend(get_ref_nodes_from_type(&mut field.ty));

                    digrphs.push(digrph);
                }
            }
        }
    }

    digrphs
}

fn get_ref_nodes_from_type<'a>(ty: &'a mut Type) -> Vec<RNode<'a>> {
    //println!("ty: {:#?}", ty);
    let mut nodes = vec![];

    match ty {
        Type::Reference(ref mut tr) => {
            tr.lifetime = Some(Lifetime::new("'null", Span::call_site()));

            nodes.push(RNode::new_lifetime(
                tr.lifetime.as_mut().unwrap(),
            ));
            nodes.extend(get_ref_nodes_from_type(&mut *tr.elem));
        }
        Type::Tuple(ref mut tt) => {
            for elem in tt.elems.iter_mut() {
                nodes.extend(get_ref_nodes_from_type(elem));
            }
        }
        Type::Path(TypePath {
            ref mut qself,
            path: ref mut pt,
            ..
        }) => {
            if let Some(qself) = qself {
                nodes.extend(get_ref_nodes_from_type(&mut *qself.ty));
            }

            nodes.extend(get_ref_nodes_from_path(pt));
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

fn get_ref_nodes_from_path<'a>(path: &'a mut Path) -> Vec<RNode<'a>> {
    let mut nodes = vec![];

    for segment in path.segments.iter_mut() {
        match segment.arguments {
            PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                ref mut args, ..
            }) => {
                for arg in args {
                    match arg {
                        GenericArgument::Type(ref mut ty) => {
                            nodes.extend(get_ref_nodes_from_type(ty));
                        }
                        GenericArgument::Binding(Binding { ref mut ty, .. }) => {
                            nodes.extend(get_ref_nodes_from_type(ty));
                        }
                        GenericArgument::Constraint(Constraint { ref mut bounds, .. }) => {
                            for bound in bounds {
                                match bound {
                                    TypeParamBound::Trait(TraitBound {
                                        path: ref mut pt, ..
                                    }) => {
                                        nodes.extend(get_ref_nodes_from_path(pt));
                                    }
                                    _ => (),
                                }
                            }
                        }
                        GenericArgument::Lifetime(lf) => {
                            nodes.push(RNode::new_lifetime(lf));
                        }
                        _ => (),
                        /*
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
                    nodes.extend(get_ref_nodes_from_type(input));
                }

                if let ReturnType::Type(_, box ref mut ty) = output {
                    nodes.extend(get_ref_nodes_from_type(ty));
                }
            }
            _ => (),
        }
    }

    nodes
}

/*
fn set_path_lfs_and_get_coords(
    pt: &mut Path,
    symbol_generator: &mut SymbolGenerator,
) -> Vec<Option<Vec<(String, u8)>>> {
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
*/

fn get_name_from_pat(pat: &Pat) -> String {
    match pat {
        Pat::Box(PatBox { box ref pat, .. }) => get_name_from_pat(pat),
        Pat::Ident(pi) => pi.ident.to_string(),
        Pat::Reference(PatReference { box ref pat, .. }) => get_name_from_pat(pat),
        Pat::Type(PatType { box ref pat, .. }) => get_name_from_pat(pat),
        _ => unreachable!(),
    }
}

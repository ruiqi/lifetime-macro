use proc_macro2::Span;
use quote::quote;
use regex::Regex;
use std::collections::HashMap;
use syn::punctuated::Punctuated;
use syn::*;

#[derive(Debug)]
pub struct LifetimeNode {
    pub lifetime: *mut Lifetime,
}

impl<'a> LifetimeNode {
    fn new(lifetime: *mut Lifetime) -> Self {
        Self { lifetime: lifetime }
    }
}

#[derive(Debug)]
pub struct SegmentNode {
    pub segment: *mut PathSegment,
    pub coords: Option<Vec<(String, u8)>>,
}

impl SegmentNode {
    fn new(segment: *mut PathSegment) -> Self {
        Self {
            segment: segment,
            coords: None,
        }
    }
}

#[derive(Debug)]
pub enum RNode {
    Lifetime(LifetimeNode),
    Segment(SegmentNode),
}

impl RNode {
    fn new_lifetime(lifetime: *mut Lifetime) -> Self {
        Self::Lifetime(LifetimeNode::new(lifetime))
    }

    fn new_segment(segment: *mut PathSegment) -> Self {
        Self::Segment(SegmentNode::new(segment))
    }
}

#[derive(Debug)]
pub struct RDigrph {
    pub name: String,
    pub nodes: Vec<RNode>,
}

impl RDigrph {
    fn new(name: String) -> Self {
        Self {
            name: name,
            nodes: vec![],
        }
    }

    pub fn get_coords(&self) -> Vec<(String, u8)> {
        let mut coords = vec![];
        let mut index_counters = HashMap::new();

        for node in self.nodes.iter() {
            match node {
                // lifetime coords
                RNode::Lifetime(_) => {
                    let index = index_counters.entry("".to_string()).or_insert(-1);
                    *index += 1;

                    // $ is end
                    coords.push((format!("{}$", self.name), *index as u8));
                }

                // segment coords
                RNode::Segment(SegmentNode {
                    segment,
                    coords: Some(cds),
                    ..
                }) => unsafe {
                    let name = (**segment).ident.to_string();
                    let index = index_counters.entry(name.clone()).or_insert(-1);
                    *index += 1;
                    let re = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*/").unwrap();

                    coords.extend(cds.iter().map(|cd| {
                        //println!("cd: {:?}", cd);
                        (
                            format!(
                                "{}{}",
                                self.name,
                                re.replace(
                                    cd.0.as_str(),
                                    format!("[{},{}].", name.clone(), index).as_str()
                                )
                            ),
                            cd.1,
                        )
                    }));
                },
                _ => (),
            }
        }

        coords
    }
}

pub enum ROrigin<'a> {
    FnInputs(&'a mut Punctuated<FnArg, token::Comma>),
    FnOutput(&'a mut ReturnType),
    StructFields(&'a mut Fields),
    EnumVariants(&'a mut Punctuated<Variant, token::Comma>),
    SelfTY(&'a mut Box<Type>),
    Trait(&'a mut Option<(Option<Token![!]>, Path, Token![for])>),
    Generics(&'a mut Generics),
}

pub fn get_ref_digrphs<'a>(namespace: String, origins: Vec<ROrigin<'a>>) -> Vec<RDigrph> {
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

                            digrph.name = format_digrph_name(namespace.clone(), "self".to_string());
                            digrph
                                .nodes
                                .push(RNode::new_lifetime(olf.as_mut().unwrap()));
                        }
                        FnArg::Typed(pt) => {
                            digrph.name = format_digrph_name(namespace.clone(), get_name_from_pat(&pt.pat));
                            digrph.nodes.extend(get_ref_nodes_from_type(&mut *pt.ty));
                        }
                        _ => (),
                    }

                    digrphs.push(digrph);
                }
            }
            ROrigin::FnOutput(output) => {
                let mut digrph = RDigrph::new(format_digrph_name(namespace.clone(), "Output!".to_string()));

                match output {
                    ReturnType::Type(_, box ty) => {
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
                    let mut digrph = RDigrph::new(format_digrph_name(namespace.clone(), field_name));

                    digrph.nodes.extend(get_ref_nodes_from_type(&mut field.ty));

                    digrphs.push(digrph);
                }
            }
            ROrigin::EnumVariants(variants) => {
                for variant in variants.iter_mut() {
                    let mut digrph =
                        RDigrph::new(format_digrph_name(namespace.clone(), variant.ident.to_string()));

                    for field in variant.fields.iter_mut() {
                        digrph.nodes.extend(get_ref_nodes_from_type(&mut field.ty));
                    }

                    digrphs.push(digrph);
                }
            }
            ROrigin::SelfTY(self_ty) => {
                let mut digrph = RDigrph::new(format_digrph_name(namespace.clone(), "self".to_string()));

                digrph
                    .nodes
                    .extend(get_ref_nodes_from_type(self_ty.as_mut()));

                digrphs.push(digrph);
            }
            ROrigin::Trait(trait_) => {
                let mut digrph = RDigrph::new(format_digrph_name(namespace.clone(), "trait".to_string()));

                match trait_ {
                    Some((_, path, _)) => digrph.nodes.extend(get_ref_nodes_from_path(path)),
                    None => (),
                }

                digrphs.push(digrph);
            }
            ROrigin::Generics(generics) => {
                for gp in generics.params.iter_mut() {
                    match gp {
                        GenericParam::Type(tp) => {
                            let mut digrph =
                                RDigrph::new(format_digrph_name(namespace.clone(), tp.ident.to_string()));

                            for tpb in tp.bounds.iter_mut() {
                                if let TypeParamBound::Trait(tb) = tpb {
                                    digrph.nodes.extend(get_ref_nodes_from_path(&mut tb.path))
                                }
                            }

                            digrphs.push(digrph);
                        }
                        GenericParam::Const(cp) => {
                            let mut digrph =
                                RDigrph::new(format_digrph_name(namespace.clone(), cp.ident.to_string()));
                            digrph.nodes.extend(get_ref_nodes_from_type(&mut cp.ty));

                            digrphs.push(digrph);
                        }
                        GenericParam::Lifetime(_) => (),
                    }
                }

                if let Some(where_clause) = &mut generics.where_clause {
                    for (i, predicate) in where_clause.predicates.iter_mut().enumerate() {
                        match predicate {
                            WherePredicate::Type(PredicateType {
                                bounded_ty, bounds, ..
                            }) => {
                                // second half
                                let mut digrph = RDigrph::new(format_digrph_name(
                                    namespace.clone(),
                                    quote!(#bounded_ty)
                                        .to_string()
                                        .split_whitespace()
                                        .collect::<String>(),
                                ));

                                for tpb in bounds.iter_mut() {
                                    if let TypeParamBound::Trait(tb) = tpb {
                                        digrph.nodes.extend(get_ref_nodes_from_path(&mut tb.path))
                                    }
                                }

                                digrphs.push(digrph);

                                // first half
                                let mut digrph =
                                    RDigrph::new(format_digrph_name(namespace.clone(), i.to_string()));

                                digrph.nodes.extend(get_ref_nodes_from_type(bounded_ty));

                                digrphs.push(digrph);
                            }
                            WherePredicate::Eq(_) => (),
                            WherePredicate::Lifetime(_) => (),
                        }
                    }
                }
            }
        }
    }

    digrphs
}

fn get_ref_nodes_from_type<'a>(ty: &'a mut Type) -> Vec<RNode> {
    //println!("ty: {:#?}", ty);
    let mut nodes = vec![];

    match ty {
        Type::Reference(tr) => {
            tr.lifetime = Some(Lifetime::new("'null", Span::call_site()));

            nodes.push(RNode::new_lifetime(tr.lifetime.as_mut().unwrap()));
            nodes.extend(get_ref_nodes_from_type(&mut *tr.elem));
        }
        Type::Tuple(tt) => {
            for elem in tt.elems.iter_mut() {
                nodes.extend(get_ref_nodes_from_type(elem));
            }
        }
        Type::Path(TypePath { qself, path, .. }) => {
            if let Some(qself) = qself {
                nodes.extend(get_ref_nodes_from_type(&mut *qself.ty));
            }

            nodes.extend(get_ref_nodes_from_path(path));
        }
        Type::BareFn(bf) => {
            //println!("BareFn: {:#?}", bf);

            // bare fn inputs
            for input in bf.inputs.iter_mut() {
                nodes.extend(get_ref_nodes_from_type(&mut input.ty));
            }

            //bare fn output
            match &mut bf.output {
                ReturnType::Type(_, box ty) => {
                    nodes.extend(get_ref_nodes_from_type(ty));
                }
                _ => (),
            }
        }
        Type::ImplTrait(it) => {
            //println!("ImplTrait: {:#?}", it);

            for bound in it.bounds.iter_mut() {
                match bound {
                    TypeParamBound::Trait(tb) => {
                        nodes.extend(get_ref_nodes_from_path(&mut tb.path));
                    }
                    TypeParamBound::Lifetime(_) => (),
                }
            }
        }
        Type::Infer(_) => (),
        _ => {
            println!("ty: {:#?}", ty);
            unreachable!()
        } /*
          Type::Array(_) => {}
          Type::Group(_) => {}
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

fn get_ref_nodes_from_path<'a>(path: &'a mut Path) -> Vec<RNode> {
    let mut nodes = vec![];

    for segment in path.segments.iter_mut() {
        nodes.push(RNode::new_segment(segment));

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

fn get_name_from_pat(pat: &Pat) -> String {
    match pat {
        Pat::Box(PatBox { box ref pat, .. }) => get_name_from_pat(pat),
        Pat::Ident(pi) => pi.ident.to_string(),
        Pat::Reference(PatReference { box ref pat, .. }) => get_name_from_pat(pat),
        Pat::Type(PatType { box ref pat, .. }) => get_name_from_pat(pat),
        _ => unreachable!(),
    }
}

fn format_digrph_name(namespace: String, name: String) -> String {
    if namespace.is_empty() {
        name
    } else {
        format!("{}/{}", namespace, name)
    }
}

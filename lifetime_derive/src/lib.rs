#![feature(box_patterns)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::collections::HashMap;
use syn::{
    parse_macro_input, AttributeArgs, FnArg, GenericParam, Item, ItemFn, ItemStruct, Lifetime,
    LifetimeDef, Lit, NestedMeta, Pat, PatType, ReturnType, Type,
};

#[proc_macro_attribute]
pub fn lifetime(args: TokenStream, input: TokenStream) -> TokenStream {
    //println!("args: {:#?}", args);
    //println!("input: {:#?}", input);

    let args = parse_macro_input!(args as AttributeArgs);

    match parse_macro_input!(input as Item) {
        Item::Struct(input) => macro_struct(args, input),
        Item::Fn(input) => macro_fn(args, input),
        _ => unreachable!(""),
    }
}

fn macro_struct(args: AttributeArgs, structure: ItemStruct) -> TokenStream {
    println!("{:#?}", structure);
    let mut generic_lifetime_symbols = Vec::new();
    let mut field_lifetime_symbols = HashMap::new();

    for (i, arg) in args.iter().enumerate() {
        if let NestedMeta::Lit(Lit::Str(arg)) = arg {
            let arg: String = arg.value().split_whitespace().collect();
            let field_idents: Vec<&str> = arg.split(",").collect();

            let symbol = format!("'_{}", (i as u8 + 97) as char);

            generic_lifetime_symbols.push(symbol.clone());
            field_lifetime_symbols.extend(
                field_idents
                    .iter()
                    .map(|&ident| (ident.to_string(), symbol.clone())),
            );
        }
    }

    println!(
        "{:#?}, {:#?}",
        generic_lifetime_symbols, field_lifetime_symbols
    );

    let expanded = quote! {
        #structure
    };

    expanded.into()
}

fn macro_fn(args: AttributeArgs, mut function: ItemFn) -> TokenStream {
    let mut generic_lifetime_symbols = Vec::new();
    let mut input_lifetime_symbols = HashMap::new();
    let mut output_lifetime_symbols = HashMap::new();

    for (i, arg) in args.iter().enumerate() {
        if let NestedMeta::Lit(Lit::Str(arg)) = arg {
            let arg: String = arg.value().split_whitespace().collect();
            let arg: Vec<&str> = arg.splitn(2, ":").collect();
            let input_idents: Vec<&str> = arg[1].split(",").collect();
            let output_indexs: Vec<&str> = arg[0].split(",").collect();

            let symbol = format!("'_{}", (i as u8 + 97) as char);

            generic_lifetime_symbols.push(symbol.clone());
            input_lifetime_symbols.extend(
                input_idents
                    .iter()
                    .map(|&ident| (ident.to_string(), symbol.clone())),
            );
            output_lifetime_symbols.extend(
                output_indexs
                    .iter()
                    .map(|&index| (index.to_string(), symbol.clone())),
            );
        }
    }

    let function_vis = &function.vis;
    let function_ident = &function.sig.ident;
    let function_generics = &mut function.sig.generics;
    let function_inputs = &mut function.sig.inputs;
    let function_output = &mut function.sig.output;
    let function_block = &function.block;

    // function generics
    for symbol in generic_lifetime_symbols {
        let lt = LifetimeDef::new(Lifetime::new(&symbol, Span::call_site()));
        function_generics.params.push(GenericParam::from(lt));
    }

    // function inputs
    for function_input in function_inputs.iter_mut() {
        //println!("function_input: {:#?}", function_input);

        match *function_input {
            FnArg::Typed(PatType {
                pat: box Pat::Ident(ref pi),
                ty: box Type::Reference(ref mut tr),
                ..
            }) => {
                let symbol = input_lifetime_symbols.get(&pi.ident.to_string());
                if symbol.is_some() {
                    tr.lifetime = Some(Lifetime::new(symbol.unwrap(), Span::call_site()));
                }
            }
            _ => (),
        }
    }

    // function output
    if let ReturnType::Type(_, function_output) = function_output {
        //println!("{:#?}", function_output);

        match **function_output {
            Type::Tuple(ref mut tt) => {
                for (i, elem) in tt.elems.iter_mut().enumerate() {
                    if let syn::Type::Reference(tr) = elem {
                        let symbol = &output_lifetime_symbols[&i.to_string()];
                        tr.lifetime = Some(Lifetime::new(symbol, Span::call_site()));
                    }
                }
            }
            Type::Reference(ref mut tr) => {
                let symbol = &output_lifetime_symbols[&0.to_string()];
                tr.lifetime = Some(Lifetime::new(symbol, Span::call_site()));
            }
            _ => (),
        }
    }

    let expanded = quote! {
        #function_vis fn #function_ident #function_generics(#function_inputs) #function_output {
            #function_block
        }
    };

    expanded.into()
}

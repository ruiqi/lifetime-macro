extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::collections::HashMap;
use syn::{
    parse_macro_input, AttributeArgs, FnArg, GenericParam, Item, ItemFn, ItemStruct, Lifetime,
    LifetimeDef, Lit, NestedMeta, Pat, ReturnType, Type,
};

#[proc_macro_attribute]
pub fn lifetime(args: TokenStream, input: TokenStream) -> TokenStream {
    //println!("args: {:#?}", args);
    //println!("input: {:#?}", input);

    let args = parse_macro_input!(args as AttributeArgs);

    match parse_macro_input!(input as Item) {
        Item::Fn(input) => macro_fn(args, input),
        Item::Struct(input) => macro_struct(args, input),
        _ => unreachable!(""),
    }
}

fn macro_fn(args: AttributeArgs, mut function: ItemFn) -> TokenStream {
    let mut generic_lifetime_symbols = Vec::new();
    let mut input_lifetime_symbols = HashMap::new();
    let mut output_lifetime_symbols = HashMap::new();

    for (i, arg) in args.iter().enumerate() {
        let mut symbol = String::from("'_");
        symbol.push((i as u8 + 97) as char);
        generic_lifetime_symbols.push(symbol.clone());

        let arg = match arg {
            NestedMeta::Lit(Lit::Str(xxx)) => xxx.value(),
            _ => unreachable!("it not gonna happen."),
        };
        let arg: String = arg.split_whitespace().collect();

        let temp: Vec<&str> = arg.split(":").collect();
        assert_eq!(temp.len(), 2);

        let input_idents: Vec<&str> = temp[1].split(",").collect();
        let output_indexs: Vec<&str> = temp[0].split(",").collect();

        for input_ident in input_idents {
            input_lifetime_symbols.insert(input_ident.to_string(), symbol.clone());
        }

        for output_index in output_indexs {
            output_lifetime_symbols.insert(output_index.to_string(), symbol.clone());
        }
    }

    /*
    println!(
        "symbols: {:#?}, {:#?}, {:#?}",
        generic_lifetime_symbols, input_lifetime_symbols, output_lifetime_symbols
    );
    */

    let function_vis = &function.vis;
    let function_ident = &function.sig.ident;
    let function_generics = &mut function.sig.generics;
    let function_inputs = &mut function.sig.inputs;
    let function_output = &mut function.sig.output;
    let function_block = &function.block;

    //println!("function_vis: {:#?}", function_vis);
    //println!("function_ident: {:#?}", function_ident);
    //println!("function_generics: {:#?}", function_generics);
    //println!("function_inputs: {:#?}", function_inputs);
    //println!("function_output: {:#?}", function_output);

    // function generics
    for symbol in generic_lifetime_symbols {
        let lt = Lifetime::new(&symbol, Span::call_site());
        let lt = LifetimeDef::new(lt);
        let lt = GenericParam::from(lt);

        function_generics.params.push(lt);
    }

    // function inputs
    for function_input in function_inputs.iter_mut() {
        //println!("function_input: {:#?}", function_input);

        if let FnArg::Typed(ref mut pt) = *function_input {
            if let Pat::Ident(ref pi) = *pt.pat {
                if let Type::Reference(ref mut tr) = *pt.ty {
                    if input_lifetime_symbols.contains_key(&pi.ident.to_string()) {
                        let symbol = &input_lifetime_symbols[&pi.ident.to_string()];
                        tr.lifetime = Some(Lifetime::new(symbol, Span::call_site()));
                    }
                }
            }
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

fn macro_struct(args: AttributeArgs, structure: ItemStruct) -> TokenStream {
    let expanded = quote! {
        #structure
    };

    expanded.into()
}

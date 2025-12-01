extern crate proc_macro;
extern crate quote;
extern crate syn;
extern crate convert_case;

use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, ItemFn};
use convert_case::{Casing, Case};

#[proc_macro_attribute]
pub fn bt_action(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let block = &input_fn.block;

    let fn_name = &sig.ident;
    let exec_name = format_ident!("{}Executor", fn_name.to_string().to_case(Case::Pascal));

    // arguments
    let args = sig.inputs.iter().collect::<Vec<_>>();

    // arguments as fields
    let fields = args.iter().map(|arg| match arg {
        syn::FnArg::Typed(pat) => {
            let name = &pat.pat;
            let ty = &pat.ty;
            quote! { #name: #ty }
        }
        _ => unimplemented!("methods not supported"),
    });

    // arguments for constructor
    let ctor_args = fields.clone();

    // passing clone() to original async fn call
    let call_args = args.iter().map(|arg| match arg {
        syn::FnArg::Typed(pat) => {
            let name = &pat.pat;
            quote! { self.#name.clone() }
        }
        _ => unimplemented!(),
    });

    let name_str = fn_name.to_string();

    let self_args = args.iter().map(|arg| match arg {
        syn::FnArg::Typed(pat) => {
            let name = &pat.pat;
            quote! { #name }
        }
        _ => unimplemented!(),
    });

    let expanded = quote! {
        #vis #sig #block

        #[derive(Clone)]
        pub struct #exec_name {
            #( #fields ),*
        }

        impl Executor for #exec_name {
            fn get_name(&self) -> String {
                #name_str.to_string()
            }

            async fn execute(&mut self) -> Result<bool, Error> {
                #fn_name( #( #call_args ),* ).await
            }
        }

        impl #exec_name {
            pub fn new( #( #ctor_args ),* ) -> #exec_name {
                Self { #( #self_args ),* }
            }
        }
    };

    expanded.into()
}
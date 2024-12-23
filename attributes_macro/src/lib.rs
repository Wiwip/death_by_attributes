extern crate proc_macro;

use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Attribute)]
pub fn derive_attribute_macro(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let st_name = input.ident;
    let (_impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let stream = quote! {
        #[automatically_derived]
        impl GameAttributeMarker for #st_name #ty_generics #where_clause {

        }

        #[automatically_derived]
        impl #st_name {
            pub fn new(value: f32) -> Self {
                Self {
                    value: GameAttribute {
                        current_value: value,
                        base_value: value,
                    }
                }
            }
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(stream)
}

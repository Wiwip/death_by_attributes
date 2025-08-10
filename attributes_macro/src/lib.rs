extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Error, Fields, Meta, Variant};


#[proc_macro_attribute]
pub fn attribute_calculator(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(item as DeriveInput);

    // Run the main generation logic. If it fails, convert the error to a compile error.
    match generate_calculator_code(input) {
        Ok(token_stream) => token_stream.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn generate_calculator_code(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    // We expect the macro to be on an enum.
    let enum_data = if let Data::Enum(ref data) = input.data {
        data
    } else {
        return Err(Error::new_spanned(
            input,
            "AttributeCalculator can only be used on enums",
        ));
    };

    let enum_name = &input.ident;
    //let generics = &input.generics;

    // A helper struct to hold the variants sorted by their category.
    struct CategorizedVariants<'a> {
        overrides: Vec<&'a Variant>,
        additives: Vec<&'a Variant>,
        increased: Vec<&'a Variant>,
        multiplicatives: Vec<&'a Variant>,
    }

    let mut categorized = CategorizedVariants {
        overrides: Vec::new(),
        additives: Vec::new(),
        increased: Vec::new(),
        multiplicatives: Vec::new(),
    };

    println!("PROC_MACRO");

    // Iterate over each variant of the enum (e.g., `BaseSet(f64)`).
    for variant in &enum_data.variants {
        // Each variant must hold a single unnamed field (e.g., `(f64)`).
        if let Fields::Unnamed(fields) = &variant.fields {
            if fields.unnamed.len() != 1 {
                return Err(Error::new_spanned(
                    variant,
                    "Variant must have exactly one unnamed field",
                ));
            }
        } else {
            return Err(Error::new_spanned(
                variant,
                "Variant must be a tuple-style variant with one field",
            ));
        }

        // Find the `#[category(...)]` attribute and classify the variant.
        let category = get_category_from_attributes(&variant.attrs)?;
        match category.as_str() {
            "set" => categorized.overrides.push(variant),
            "additive" => categorized.additives.push(variant),
            "increased" => categorized.increased.push(variant),
            "multiplicative" => categorized.multiplicatives.push(variant),
            _ => {
                return Err(Error::new_spanned(
                    variant,
                    format!("Unknown category: '{}'", category),
                ));
            }
        }
    }

    // Generate the enum without the custom attributes
    let variants_without_attrs: Vec<_> = enum_data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let fields = &variant.fields;

        // Remove our custom attributes
        let filtered_attrs: Vec<_> = variant.attrs.iter()
            .filter(|attr| !is_custom_attribute(attr))
            .collect();

        quote! {
            #(#filtered_attrs)*
            #variant_name #fields
        }
    }).collect();

    // --- 2. CODE GENERATION ---

    // Generate the names for the new structs based on the input enum's name.
    let calculator_name = format_ident!("{}Calculator", enum_name);
    let aggregator_name = format_ident!("{}Aggregator", enum_name);

    // Helper to handle generics correctly.
    //let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Generate the match arms for the `from_iter` method.
    let override_arms = categorized.overrides.iter().map(|v| {
        let variant_name = &v.ident;
        quote! { #enum_name::#variant_name(value) => { final_set = Some(*value); } }
    });
    let additive_arms = categorized.additives.iter().map(|v| {
        let variant_name = &v.ident;
        quote! { #enum_name::#variant_name(value) => { total_additive += value; } }
    });
    let increased_arms = categorized.increased.iter().map(|v| {
        let variant_name = &v.ident;
        quote! { #enum_name::#variant_name(value) => { total_increased += value; } }
    });
    let multiplicative_arms = categorized.multiplicatives.iter().map(|v| {
        let variant_name = &v.ident;
        quote! { #enum_name::#variant_name(value) => { total_multiplicative *= (1.0 + value); } }
    });

    // Use the `quote!` macro to build the final TokenStream.
    let generated_code = quote! {
        // Pass through the original enum definition untouched.
        #[derive(Debug, Clone)]
        pub enum #enum_name {
            #(#variants_without_attrs),*
        }


        // --- GENERATED CALCULATOR STRUCT ---
        #[derive(Debug, Clone)]
        pub struct #calculator_name {
            pub set: Option<f64>,
            pub additive: f64,
            pub increased: f64,
            pub multiplicative: f64,
        }

        impl #calculator_name {
            pub fn calculate(&self, base_value: f64) -> f64 {
                if let Some(set_value) = self.set {
                    return set_value;
                }

                let after_additive = base_value + self.additive;
                let after_increased = after_additive * (1.0 + self.increased);
                let after_multiplicative = after_increased * self.multiplicative;

                after_multiplicative
            }
        }

        // --- GENERATED AGGREGATOR STRUCT ---
        pub struct #aggregator_name;

        impl #aggregator_name {
            pub fn from_iter<'a>(modifiers: impl IntoIterator<Item = &'a #enum_name>) -> #calculator_name {
                let mut final_set: Option<f64> = None;
                let mut total_additive: f64 = 0.0;
                let mut total_increased: f64 = 0.0;
                let mut total_multiplicative: f64 = 1.0;

                for modifier in modifiers {
                    match modifier {
                        #( #override_arms )*
                        #( #additive_arms )*
                        #( #increased_arms )*
                        #( #multiplicative_arms )*
                    }
                }

                #calculator_name {
                    set: final_set,
                    additive: total_additive,
                    increased: total_increased,
                    multiplicative: total_multiplicative,
                }
            }
        }
    };

    Ok(TokenStream::from(generated_code))
}

fn is_custom_attribute(attr: &Attribute) -> bool {
    if let Meta::Path(path) = &attr.meta {
        path.is_ident("set") ||
            path.is_ident("additive") ||
            path.is_ident("multiplicative")
    } else {
        false
    }
}


/// Helper function to parse `#[category(...)]` attributes.
fn get_category_from_attributes(attrs: &[Attribute]) -> Result<String, Error> {
    for attr in attrs {
        println!("attr: {:?}", attr.path().get_ident());
        if attr.path().is_ident("set")
            || attr.path().is_ident("additive")
            || attr.path().is_ident("multiplicative")
        {
            return Ok(attr
                .path()
                .get_ident()
                .expect("Expected `category`")
                .to_string());
        }
    }
    Err(Error::new_spanned(
        attrs.first().expect("Expected at least one attribute"),
        "Missing required `#[category(...)]` attribute",
    ))
}

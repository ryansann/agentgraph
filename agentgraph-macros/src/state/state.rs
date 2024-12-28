use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub fn derive_state_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let update_name = format_ident!("{}Update", name);

    // Extract fields and their update strategies
    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    // Generate update enum and implementations
    let mut update_variants = vec![];
    let mut update_match_arms = vec![];

    for field in fields {
        let field_name = field.ident.unwrap();
        let field_type = field.ty;

        // Parse update attribute
        let update_strategy = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("update"))
            .map(|attr| attr.parse_args::<syn::Ident>().unwrap())
            .unwrap_or_else(|| syn::Ident::new("replace", proc_macro2::Span::call_site()));

        let variant_name = format_ident!("{}", field_name.to_string().to_case(Case::Pascal));
        update_variants.push(quote! {
            #variant_name(#field_type)
        });

        let update_impl = match update_strategy.to_string().as_str() {
            "append" => quote! { self.#field_name.extend(value) },
            "merge" => quote! { self.#field_name.extend(value.into_iter()) },
            "replace" => quote! { self.#field_name = value },
            strategy => panic!("Unknown update strategy: {}", strategy),
        };

        update_match_arms.push(quote! {
            #update_name::#variant_name(value) => { #update_impl }
        });
    }

    let expanded = quote! {
        #[derive(Debug)]
        pub enum #update_name {
            #(#update_variants),*
        }

        impl ::agentgraph_core::GraphState for #name {
            type Update = #update_name;

            fn apply(&mut self, update: Self::Update) {
                match update {
                    #(#update_match_arms),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

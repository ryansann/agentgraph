use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, Error, FnArg, ItemFn, LitStr, PatType, ReturnType, Type,
    TypePath, PathArguments, GenericArgument, Receiver,
};

pub fn tool_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let description = parse_macro_input!(attr as LitStr);
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    let vis = &input_fn.vis;

    // Parse the function parameters and determine if it's a method
    let mut has_receiver = false;
    let mut tool_type = None;
    let mut params_type = None;

    for arg in &input_fn.sig.inputs {
        match arg {
            FnArg::Receiver(Receiver { reference: Some(_), mutability: None, .. }) => {
                has_receiver = true;
            }
            FnArg::Receiver(_) => {
                return Error::new(
                    arg.span(),
                    "Tool methods must take &self (not mut or owned self)",
                ).to_compile_error().into();
            }
            FnArg::Typed(pat_type) => {
                if !has_receiver && tool_type.is_none() {
                    match &*pat_type.ty {
                        Type::Reference(ref_type) => {
                            tool_type = Some((*ref_type.elem).clone());
                        }
                        _ => {
                            return Error::new(
                                pat_type.ty.span(),
                                "First argument must be a reference type",
                            ).to_compile_error().into();
                        }
                    }
                } else {
                    params_type = Some((*pat_type.ty).clone());
                    break;
                }
            }
        }
    }

    // Get the parameters type
    let params_type = params_type.ok_or_else(|| Error::new(
        input_fn.span(),
        "Missing params argument",
    )).unwrap();

    // Get the return type
    let return_type = match &input_fn.sig.output {
        ReturnType::Default => {
            return Error::new(
                input_fn.sig.output.span(),
                "Tool function must return a concrete type or Result<T, E>.",
            ).to_compile_error().into();
        }
        ReturnType::Type(_, ty) => (*ty).clone(),
    };

    let (success_type, needs_question_mark) = parse_success_type(*return_type);

    // Generate different expansions for methods vs standalone functions
    let expanded = if has_receiver {
        quote! {
            #[derive(Clone)]
            struct Tool;

            impl Tool {
                fn new() -> Self {
                    Self
                }
            }

            #[automatically_derived]
            #[async_trait::async_trait]
            impl ToolFunction for Tool {
                type Params = #params_type;
                type Response = #success_type;

                fn name() -> &'static str {
                    #fn_name_str
                }

                fn description() -> &'static str {
                    #description
                }

                async fn execute(&self, params: Self::Params) 
                    -> std::result::Result<Self::Response, ToolError>
                {
                    Ok(#fn_name(params).await?)
                }
            }

            #input_fn
        }
    } else {
        let tool_type = tool_type.unwrap();
        quote! {
            #[automatically_derived]
            #[async_trait::async_trait]
            impl ToolFunction for #tool_type {
                type Params = #params_type;
                type Response = #success_type;

                fn name() -> &'static str {
                    #fn_name_str
                }

                fn description() -> &'static str {
                    #description
                }

                async fn execute(&self, params: Self::Params) 
                    -> std::result::Result<Self::Response, ToolError>
                {
                    Ok(#fn_name(self, params).await?)
                }
            }

            #input_fn
        }
    };

    TokenStream::from(expanded)
}

fn parse_success_type(ty: Type) -> (Type, bool) {
    if let Type::Path(TypePath { path, .. }) = &ty {
        if let Some(last) = path.segments.last() {
            if last.ident == "Result" {
                if let PathArguments::AngleBracketed(args) = &last.arguments {
                    let mut generic_args = args.args.iter();
                    let first = generic_args.next();
                    let second = generic_args.next();
                    if let (Some(GenericArgument::Type(success)), Some(_)) = (first, second) {
                        return (success.clone(), true);
                    }
                }
            }
        }
    }
    (ty, false)
}
// Helper functions
fn extract_param_type(input_fn: &ItemFn) -> Option<Type> {
    let mut param_types = input_fn.sig.inputs.iter().filter_map(|arg| {
        match arg {
            FnArg::Typed(PatType { ty, .. }) => Some((**ty).clone()),
            FnArg::Receiver(_) => None,
        }
    });

    param_types.next()
}

fn extract_return_type(input_fn: &ItemFn) -> Option<Type> {
    match &input_fn.sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => Some((**ty).clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_extract_param_type() {
        let input: ItemFn = parse_quote! {
            async fn test_fn(params: TestParams) -> TestResponse {
                unimplemented!()
            }
        };

        let param_type = extract_param_type(&input);
        assert!(param_type.is_some());
        
        // Test with no params
        let input_no_params: ItemFn = parse_quote! {
            async fn test_fn() -> TestResponse {
                unimplemented!()
            }
        };
        
        let param_type = extract_param_type(&input_no_params);
        assert!(param_type.is_none());
        
        // Test with self param
        let input_with_self: ItemFn = parse_quote! {
            async fn test_fn(&self, params: TestParams) -> TestResponse {
                unimplemented!()
            }
        };
        
        let param_type = extract_param_type(&input_with_self);
        let param_type_str = param_type.unwrap().to_token_stream().to_string();
        assert_eq!(param_type_str, "TestParams");
    }

    #[test]
    fn test_extract_return_type() {
        let input: ItemFn = parse_quote! {
            async fn test_fn(params: TestParams) -> TestResponse {
                unimplemented!()
            }
        };

        let return_type = extract_return_type(&input);
        let return_type_str = return_type.unwrap().to_token_stream().to_string();
        assert_eq!(return_type_str, "TestResponse");

        // Test with Result return type
        let input_result: ItemFn = parse_quote! {
            async fn test_fn(params: TestParams) -> Result<TestResponse, Error> {
                unimplemented!()
            }
        };

        let return_type = extract_return_type(&input_result);
        let return_type_str = return_type.unwrap().to_token_stream().to_string();
        assert_eq!(return_type_str, "Result < TestResponse , Error >");

        // Test with no return type
        let input_no_return: ItemFn = parse_quote! {
            async fn test_fn(params: TestParams) {
                unimplemented!()
            }
        };

        let return_type = extract_return_type(&input_no_return);
        assert!(return_type.is_none());
    }
    
    // Helper function to normalize whitespace in strings
    fn normalize_ws(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

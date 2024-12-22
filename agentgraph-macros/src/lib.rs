use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, spanned::Spanned, Error, FnArg, ItemFn, LitStr, PatType, ReturnType, Type,
    TypePath, PathArguments, GenericArgument,
};

#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let description = parse_macro_input!(attr as LitStr);
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();

    // We expect exactly two typed parameters: `&_tool: &ToolStruct`, `params: ParamsType`
    let mut typed_args: Vec<&PatType> = input_fn
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Typed(t) => Some(t),
            FnArg::Receiver(_) => None,
        })
        .collect();
    if typed_args.len() != 2 {
        return Error::new(
            input_fn.sig.span(),
            "Tool function must have exactly two arguments: `&_tool: &ToolStruct`, `params: ParamType`",
        )
        .to_compile_error()
        .into();
    }

    // First argument => &ToolStruct
    let tool_struct_type = match &*typed_args[0].ty {
        Type::Reference(ref_type) => ref_type.elem.clone(),
        _ => {
            return Error::new(
                typed_args[0].ty.span(),
                "First argument must be a reference to a tool struct, e.g. &MyTool.",
            )
            .to_compile_error()
            .into();
        }
    };

    // Second argument => ParamType
    let params_type = typed_args[1].ty.clone();

    // Return type: unbox the Box<Type> to get a Type
    let return_type = match &input_fn.sig.output {
        ReturnType::Default => {
            return Error::new(
                input_fn.sig.output.span(),
                "Tool function must return a concrete type or Result<T, E>.",
            )
            .to_compile_error()
            .into();
        }
        ReturnType::Type(_, ty) => (*ty).clone(),
    };

    // Inspect if it's Result<T, E> or just T
    let (success_type, is_result) = parse_success_type(*return_type);

    // If the user returns `Result<T, E>`, we do:
    //   Ok(#fn_name(self, params).await?)
    // If the user returns T, we do:
    //   Ok(#fn_name(self, params).await)
    let call_stmt = if is_result {
        quote! {
            Ok(#fn_name(self, params).await?)
        }
    } else {
        quote! {
            Ok(#fn_name(self, params).await)
        }
    };

    let expanded = quote! {
        #[async_trait::async_trait]
        impl ToolFunction for #tool_struct_type {
            type Params = #params_type;
            type Response = #success_type;

            fn name() -> &'static str {
                #fn_name_str
            }

            fn description() -> &'static str {
                #description
            }

            async fn execute(
                &self,
                params: Self::Params
            ) -> std::result::Result<Self::Response, ToolError> {
                #call_stmt
            }
        }

        // Keep the user's original function
        #input_fn
    };

    expanded.into()
}

/// Parse out the success type from `-> Result<T, E>` or a plain type.
/// Returns `(T, bool)` where the bool is `true` if it's a `Result<T, E>`.
fn parse_success_type(ty: Type) -> (Type, bool) {
    // If it's a `Result<..., ...>` path, parse out the success type
    if let Type::Path(TypePath { path, .. }) = &ty {
        if let Some(last) = path.segments.last() {
            if last.ident == "Result" {
                // e.g. `Result<T, E>`
                if let PathArguments::AngleBracketed(args) = &last.arguments {
                    // We expect exactly two generic args
                    let mut generic_args = args.args.iter();
                    let first = generic_args.next();
                    let second = generic_args.next();
                    if let (Some(GenericArgument::Type(success)), Some(_)) = (first, second) {
                        // success is T
                        return (success.clone(), true);
                    }
                }
            }
        }
    }
    // Otherwise, just return the whole type, with false indicating "not a Result"
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
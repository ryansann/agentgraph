use proc_macro::TokenStream;
use proc_macro2::TokenStream as Pm2TokenStream;
use proc_macro::TokenStream as PmTokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_macro_input,
    parse::Parse, parse::ParseStream,
    token::Comma, 
    punctuated::Punctuated,
    Error, Expr, ExprLit, FnArg, ImplItem, Item, ItemImpl, ImplItemFn, Lit, MetaNameValue,
    ReturnType, Type,
    Result as SynResult,
};

/// A simple wrapper that can parse comma-separated `MetaNameValue` items.
///
/// For example, this can parse:
/// `add = "desc", subtract = "desc2"`
/// into a list of `MetaNameValue`s.
struct ToolsAttribute {
    name_values: Punctuated<MetaNameValue, Comma>,
}

impl Parse for ToolsAttribute {
    fn parse(input: ParseStream) -> SynResult<Self> {
        // Parse as a comma-delimited list of MetaNameValue
        let name_values = Punctuated::<MetaNameValue, Comma>::parse_terminated(input)?;
        Ok(Self { name_values })
    }
}

/// Entry point for the `#[tools(...)]` attribute macro.
/// 
/// Usage example:
/// ```
/// #[tools(add = "Adds two numbers", subtract = "Subtracts b from a")]
/// impl MathTool {
///     async fn add(&self, params: AddParams) -> Result<AddResponse, ToolError> { ... }
///     async fn subtract(&self, params: SubParams) -> Result<SubResponse, ToolError> { ... }
/// }
/// ```
pub fn tools_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    // 1. Parse the attribute as name-value pairs
    let tool_map = match parse_tools_map(attr) {
        Ok(map) => map,
        Err(err) => return err.to_compile_error().into(),
    };

    // 2. Parse the item as `Item::Impl`
    let parsed_item = syn::parse_macro_input!(item as Item);
    let mut item_impl = match parsed_item {
        Item::Impl(ii) => ii,
        other => {
            return syn::Error::new_spanned(
                other,
                "#[tools(...)] can only be applied to an impl block.",
            )
            .to_compile_error()
            .into();
        }
    };

    // 3. For each `(method_name, description)` pair, find the method, generate expansions, etc.
    let tool_type = &*item_impl.self_ty;
    let mut expansions = Vec::new();

    for (method_name, description) in tool_map {
        // find the method
        let Some(method_fn) = find_method_by_name(&item_impl.items, &method_name) else {
            let msg = format!("Method `{}` not found in impl block.", method_name);
            expansions.push(syn::Error::new_spanned(&item_impl, msg).to_compile_error());
            continue;
        };

        // parse param + return type
        let (params_ty, success_ty, is_result) = match parse_param_and_return(method_fn) {
            Ok(pair) => pair,
            Err(err) => {
                expansions.push(err.to_compile_error());
                continue;
            }
        };

        let struct_name = syn::Ident::new(
            &format!("{}{}", type_to_ident_str(tool_type), capitalize(&method_name)),
            method_fn.sig.ident.span(),
        );
        let method_ident = &method_fn.sig.ident;

        // If it's a `Result<T, E>`, we do `.await?`, else `.await`
        let call_stmt = if is_result {
            quote! {
                Ok(self.0.#method_ident(params).await?)
            }
        } else {
            quote! {
                Ok(self.0.#method_ident(params).await)
            }
        };

        let expanded_one = quote! {
            #[derive(Clone)]
            pub struct #struct_name(pub #tool_type);

            #[async_trait::async_trait]
            impl ToolFunction for #struct_name {
                type Params = #params_ty;
                type Response = #success_ty;

                fn name() -> &'static str { #method_name }
                fn description() -> &'static str { #description }

                async fn execute(
                    &self,
                    params: Self::Params
                ) -> std::result::Result<Self::Response, ToolError> {
                    #call_stmt
                }
            }
        };
        expansions.push(expanded_one);
    }

    let final_ts = quote! {
        #item_impl
        #(#expansions)*
    };
    final_ts.into()
}

/// Parse the macro attribute as a comma-separated list of `MetaNameValue` pairs.
/// For example: `#[tools(add = "Adds two numbers", subtract = "Subtracts second from first")]`.
fn parse_tools_map(attr_ts: PmTokenStream) -> Result<Vec<(String, String)>, Error> {
    // 1. Convert the proc_macro TokenStream into a proc_macro2 TokenStream
    let pm2_ts = Pm2TokenStream::from(attr_ts);

    // 2. Parse it as `ToolsAttribute`
    let parsed = syn::parse2::<ToolsAttribute>(pm2_ts)?;

    // 3. Convert those MetaNameValue items into `(String, String)` pairs
    let mut result = Vec::new();
    for nv in parsed.name_values {
        // nv.path, nv.value are the left and right sides of `name = "..."`.
        let ident = nv
            .path
            .get_ident()
            .ok_or_else(|| Error::new_spanned(&nv.path, "Expected an identifier on left side"))?
            .to_string();

        // We want the right side to be a string literal
        if let syn::Expr::Lit(expr_lit) = &nv.value {
            if let syn::Lit::Str(s) = &expr_lit.lit {
                let desc = s.value();
                result.push((ident, desc));
                continue;
            }
        }
        return Err(Error::new_spanned(
            &nv.value,
            "Expected a string literal on right side (e.g. `foo = \"desc\"`)",
        ));
    }
    Ok(result)
}

/// Locate the `ImplItemFn` with the given name in the impl block.
fn find_method_by_name<'a>(items: &'a [ImplItem], name: &str) -> Option<&'a ImplItemFn> {
    for itm in items {
        if let ImplItem::Fn(m) = itm {
            if m.sig.ident == name {
                return Some(m);
            }
        }
    }
    None
}

/// Return (params_type, success_type, is_result), where:
/// - `params_type` is the type of the method's second argument (besides `&self`)
/// - `success_type` is either T or the T from `Result<T, E>`
/// - `is_result` is `true` if the user returns `Result<T, E>`, else false.
fn parse_param_and_return(m: &ImplItemFn) -> Result<(Type, Type, bool), Error> {
    // 1) Find the typed param (besides &self)
    let mut found_param_ty = None;
    for arg in &m.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            // pat_type.ty is a Box<Type>
            let ty = *pat_type.ty.clone();
            found_param_ty = Some(ty);
            break;
        }
    }
    let params_ty = found_param_ty.ok_or_else(|| {
        Error::new_spanned(
            m,
            "Method must have a typed parameter, e.g. `fn foo(&self, params: X) -> ...`"
        )
    })?;

    // 2) Extract the final success type from the method's return.
    //    If it's `Result<T, E>`, we parse out `T` and set is_result = true.
    //    If it's plain T, we set is_result = false.
    let return_ty = match &m.sig.output {
        ReturnType::Default => {
            return Err(Error::new_spanned(
                m,
                "Method must return a type (like T or Result<T, E>)",
            ));
        }
        ReturnType::Type(_, box_ty) => *box_ty.clone(),
    };

    let (success_ty, is_result) = parse_success_type(return_ty)?;
    Ok((params_ty, success_ty, is_result))
}

/// If the user returns `Result<T, E>`, parse out `T` and return `(T, true)`.
/// If the user returns a plain type `T`, return `(T, false)`.
fn parse_success_type(ty: Type) -> Result<(Type, bool), Error> {
    if let Type::Path(type_path) = &ty {
        if let Some(last_seg) = type_path.path.segments.last() {
            // e.g. `Result<T, E>` has the last segment named "Result"
            if last_seg.ident == "Result" {
                // Check generics for the success type
                if let syn::PathArguments::AngleBracketed(ref generic_args) = last_seg.arguments {
                    let mut args_iter = generic_args.args.iter();
                    let first_arg = args_iter.next();
                    let second_arg = args_iter.next();

                    // Expect `Result<SuccessType, ToolError>` shape
                    if let (Some(syn::GenericArgument::Type(success)), Some(_)) = (first_arg, second_arg)
                    {
                        // success is T
                        return Ok((success.clone(), true));
                    }
                }
            }
        }
    }
    // Else, it's a plain type
    Ok((ty, false))
}

/// Convert a type to a sanitized string we can embed in an identifier.
fn type_to_ident_str(ty: &Type) -> String {
    let s = ty.to_token_stream().to_string();
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

/// Capitalize the first letter of a string, e.g. "add" -> "Add"
fn capitalize(s: &str) -> String {
    if let Some(first) = s.chars().next() {
        first.to_uppercase().collect::<String>() + &s[1..]
    } else {
        String::new()
    }
}

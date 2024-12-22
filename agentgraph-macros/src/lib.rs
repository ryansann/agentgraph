use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    crate::tool::tool_impl(attr, item)
}

#[proc_macro_attribute]
pub fn tools(attr: TokenStream, item: TokenStream) -> TokenStream {
    crate::tools::tools_impl(attr, item)
}

mod tool;
mod tools;

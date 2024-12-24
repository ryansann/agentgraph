use proc_macro::TokenStream;

mod state;
mod tool;
mod tools;

#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    tool::tool_impl(attr, item)
}

#[proc_macro_attribute]
pub fn tools(attr: TokenStream, item: TokenStream) -> TokenStream {
    tools::tools_impl(attr, item)
}

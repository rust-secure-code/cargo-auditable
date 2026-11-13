extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro]
pub fn derive(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}

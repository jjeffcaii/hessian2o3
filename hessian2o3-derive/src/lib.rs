use proc_macro::TokenStream;

#[proc_macro_derive(HessianObject, attributes(hessian))]
pub fn derive_hessian_object(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}

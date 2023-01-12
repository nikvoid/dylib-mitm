use proc_macro::TokenStream;

mod mitm;

#[proc_macro]
pub fn dylib_mitm(args: TokenStream) -> TokenStream {
    mitm::impl_dylib_mitm(args)
}
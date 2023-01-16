use proc_macro::TokenStream;

mod mitm;

#[proc_macro]
pub fn dylib_mitm_specified(args: TokenStream) -> TokenStream {
    mitm::impl_dylib_mitm_specified(args)
}
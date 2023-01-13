use proc_macro::TokenStream;
use syn::{LitStr, parse_macro_input};
use quote::quote;

mod mitm;

#[proc_macro]
pub fn dylib_mitm(args: TokenStream) -> TokenStream {
    let lib_name = parse_macro_input!(args as LitStr);
    quote! {
        // Pass actual target os and arch to macro
        #[cfg(all(windows, target_arch = "x86"))]
        dylib_mitm::dylib_mitm_specified!(os = "windows", arch = "x86", target_lib = #lib_name);
        #[cfg(all(windows, target_arch = "x86_64"))]
        dylib_mitm::dylib_mitm_specified!(os = "windows", arch = "x86_64", target_lib = #lib_name);

        // Make macro panic
        #[cfg(not(windows))]
        dylib_mitm::dylib_mitm_specified!(os = "?", arch = "?", target_lib = #lib_name);
    }.into()
}

#[proc_macro]
pub fn dylib_mitm_specified(args: TokenStream) -> TokenStream {
    mitm::impl_dylib_mitm(args)
}
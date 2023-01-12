//! This crate intended to make dynamic libs proxying easier.
//!
//! Currently supported targets:
//! - Windows x64
use proc_macro::TokenStream;

mod mitm;

/// Load specified dynamic library and reexport it`s exports.
///
/// Library must be loaded with `lib_name::init()` as soon as possible.
///
/// Original library symbols are prefixed with `__` e.g. `__orig_symbol`.
/// # Examples
/// ```
/// // Typical code injection through d3d9 dll
/// use dylib_mitm::dylib_mitm;
/// dylib_mitm!("C:\\Windows\\system32\\d3d9.dll");
///
/// pub extern "C" fn DllMain(_: *mut u8, call_reason: i32, _: *mut u8) {
///     println!("Called dllmain of mitm DLL!");
///     match call_reason {
///         1 => d3d9::init(),
///         _ => (),
///     }
/// }
/// ```
#[proc_macro]
pub fn dylib_mitm(args: TokenStream) -> TokenStream {
    mitm::impl_dylib_mitm(args)
}
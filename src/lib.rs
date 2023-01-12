//! This crate intended to make dynamic libs proxying easier.
//!
//! Currently supported targets:
//! - Windows x64

#[allow(unused_imports)]
#[macro_use]
extern crate mitm_macro;

/// Load specified dynamic library and reexport it`s exports.
///
/// Library must be loaded with `lib_name::init()` as soon as possible.
///
/// Original library symbols are prefixed with `__` e.g. `__orig_symbol`.
/// # Examples
/// ```
/// # use mitm_macro::dylib_mitm;
/// // Typical code injection through d3d9 dll
/// dylib_mitm!("C:\\Windows\\system32\\d3d9.dll");
///
/// pub extern "C" fn DllMain(_: *mut u8, call_reason: i32, _: *mut u8) {
///     println!("Called dllmain of mitm DLL!");
///     match call_reason {
///         1 => unsafe { d3d9::init() },
///         _ => (),
///     }
/// }
/// ```
#[doc(inline)]
pub use mitm_macro::dylib_mitm;

#[doc(hidden)]
pub use libloading;
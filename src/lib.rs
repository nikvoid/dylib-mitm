//! This crate intended to make dynamic libs proxying easier.
//!
//! Currently supported targets:
//! - Windows x64
//! - Windows x86

#[allow(unused_imports)]
#[macro_use]
extern crate mitm_macro;

/// Load specified dynamic library and reexport it`s exports.
///
/// Library must be loaded with `lib_name::init()` as soon as possible.
///
/// Original library symbols are prefixed with `__` e.g. `__orig_symbol`.
/// # Examples
/// By default, library will be loaded from the same path as prototype to get symbols:
/// ```
/// # use mitm_macro::dylib_mitm;
/// // Typical code injection through d3d9 dll
/// dylib_mitm!(proto_path = "C:\\Windows\\system32\\d3d9.dll");
///
/// pub extern "C" fn DllMain(_: *mut u8, call_reason: i32, _: *mut u8) {
///     println!("Called dllmain of mitm DLL!");
///     match call_reason {
///         1 => unsafe { d3d9::init() },
///         _ => (),
///     }
/// }
/// ```
/// Also, path to library to load may be specified with arbitrary expression,
/// that returns `&str`:
/// ```no_test
/// // Can be used on WINE
/// dylib_mitm!(
///     proto_path = "path/to/wine's/d3d9.dll",
///     load_lib = r#" "C:\\Windows\\system32\\d3d9.dll" "#
/// );
/// ```

#[macro_export]
macro_rules! dylib_mitm {
    ($($args:tt)*) => {    
        // Pass actual target os and arch to macro
        #[cfg(all(windows, target_arch = "x86"))]
        dylib_mitm::dylib_mitm_specified!(
            os = "windows",
            arch = "x86",
            $($args)*
        );
        #[cfg(all(windows, target_arch = "x86_64"))]
        dylib_mitm::dylib_mitm_specified!(
            os = "windows",
            arch = "x86_64",
            $($args)*
        );

        // Make macro panic
        #[cfg(not(windows))]
        compile_error!("unsupported target");
    }    
}

/// Does the same thing as [dylib_mitm], but needs target arch and os to be specified
///
/// Actually [dylib_mitm] is just a wrapper that passes os and arch based on build target
/// # Example
/// ```no_test
/// dylib_mitm_specified!(os = "windows", arch = "x86_64", proto_path = "C:\\Windows\\system32\\d3d9.dll");
/// ```
#[doc(inline)]
pub use mitm_macro::dylib_mitm_specified;

#[doc(hidden)]
pub use libloading;
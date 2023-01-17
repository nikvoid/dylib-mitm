# dylib-mitm
This crate intended to make dynamic libs proxying easier.

Currently supported targets:
- Windows x64
- Windows x86

## Important
Default calling convention is `extern "C"`.

Dynamic libraries that export non-callable symbols (data) are not supported.
It must be rare for dylibs though.

## Example
Load specified dynamic library and reexport it's exports.
Library must be loaded with `lib_name::init()` as soon as possible.

Typical code injection through d3d9 dll:
```rust
dylib_mitm::dylib_mitm!(proto_path = r"C:\Windows\system32\d3d9.dll");

pub extern "C" fn DllMain(_: *mut u8, call_reason: i32, _: *mut u8) {
    println!("Called dllmain of mitm DLL!");
    match call_reason {
        1 => unsafe { d3d9::init() },
        _ => (),
    }
}
```
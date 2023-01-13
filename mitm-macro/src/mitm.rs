use syn::{Ident, AttributeArgs, parse_macro_input};
use proc_macro2::Span;
use quote::quote;

use pelite::util::CStr;
use pelite::FileMap;

use darling::FromMeta;

#[derive(Debug, FromMeta)]
struct DylibMitmArgs {
    os: String,
    arch: String,
    target_lib: String
}

fn get_proc_names_win32(dll_img: &[u8]) -> pelite::Result<Vec<pelite::Result<&CStr>>> {
    use pelite::pe32::{Pe, PeFile};

    let pe = PeFile::from_bytes(dll_img)?;
    Ok(pe.exports()?.by()?.iter_names().map(|(name, _)| name).collect())
}

fn get_proc_names_win64(dll_img: &[u8]) -> pelite::Result<Vec<pelite::Result<&CStr>>> {
    use pelite::pe64::{Pe, PeFile};

    let pe = PeFile::from_bytes(dll_img)?;
    Ok(pe.exports()?.by()?.iter_names().map(|(name, _)| name).collect())
}

pub fn impl_dylib_mitm(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let crate_name = Ident::new("dylib_mitm", Span::mixed_site());

    let args = parse_macro_input!(args as AttributeArgs);

    let args = match DylibMitmArgs::from_list(&args) {
        Ok(v) => v,
        Err(e) => { return proc_macro::TokenStream::from(e.write_errors()); }
    };

    let lib_path = std::path::Path::new(&args.target_lib);

    let dll = FileMap::open(&args.target_lib).expect("Failed to open dll");

    let procs = match (args.os.as_str(), args.arch.as_str()) {
        ("windows", "x86") =>
            get_proc_names_win32(dll.as_ref()).expect("Failed to get proc names"),
        ("windows", "x86_64") =>
            get_proc_names_win64(dll.as_ref()).expect("Failed to get proc names"),
        _ => panic!("Current target unsupported")
    };

    let procs = procs.into_iter().enumerate().map(|(idx, res)| match res {
        Ok(name) => name.to_str().expect("Failed to convert proc to str"),
        Err(e) => panic!("Failed to get name of export #{idx}: {e}"),
    });

    let export_idents = procs.clone().map(|name| Ident::new(name, Span::call_site()));

    let sym_idents: Vec<_> = procs.clone().map(|name| {
        let mut sym_name = "__".to_string();
        sym_name += name;
        Ident::new(&sym_name, Span::call_site())
    }).collect();

    let mut lib_name = lib_path.file_name()
        .map(|n|
            n.to_str().map(|n| n.split('.').next().unwrap())
        ).flatten().expect("Failed to get dylib name").to_string();

    lib_name = lib_name.replace('-', "_");

    let lib_struct = Ident::new(&lib_name, Span::call_site());

    lib_name += "_LIB";
    let lib_ident = Ident::new(&lib_name.to_uppercase(), Span::call_site());

    let lib_path = lib_path.to_string_lossy();

    quote! {
        #[allow(non_upper_case_globals)]
        static mut #lib_ident: Option<#lib_struct> = None;

        #(
            #[allow(non_upper_case_globals)]
            static mut #sym_idents: unsafe fn() = || {};
        )*

        #[allow(non_camel_case_types)]
        struct #lib_struct(#crate_name::libloading::Library);

        impl #lib_struct {
            pub unsafe fn init() {
                #lib_ident = Some(Self(#crate_name::libloading::Library::new(#lib_path).expect("Failed to load library")));
                let Some(#lib_struct(ref lib)) = #lib_ident else { unreachable!() };

                #( #sym_idents = *lib.get(#procs.as_bytes()).expect("Failed to get symbol"); )*
            }
        }

        #(
            #[allow(non_snake_case)]
            #[no_mangle]
            pub unsafe extern "C" fn #export_idents() {
                // For some reason intel syntax doesn't do it right...
                // See https://users.rust-lang.org/t/asm-how-to-do-a-memory-indirect-jump-using-x86-asm/67352
                #[cfg(target_arch = "x86")]
                std::arch::asm!(
                    // Just Works
                    "jmpl *({proc})",
                    proc = sym #sym_idents,
                    options(att_syntax, noreturn, nostack)
                );

                #[cfg(target_arch = "x86_64")]
                std::arch::asm!(
                    // RIP-relative addressing needed
                    "jmpq *{proc}(%rip)",
                    proc = sym #sym_idents,
                    options(att_syntax, noreturn, nostack)
                );

         }
        )*
    }.into()
}
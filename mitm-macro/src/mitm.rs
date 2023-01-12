use pelite::pe64::{Pe, PeFile};
use pelite::util::CStr;
use pelite::FileMap;
use syn::Ident;
use proc_macro2::Span;
use quote::quote;



fn get_proc_names(dll_img: &[u8]) -> pelite::Result<Vec<pelite::Result<&CStr>>> {
    let pe = PeFile::from_bytes(dll_img)?;

    Ok(pe.exports()?.by()?.iter_names().map(|(name, _)| name).collect())
}

pub fn impl_dylib_mitm(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let crate_name = Ident::new("dylib_mitm", Span::mixed_site());

    let name = syn::parse_macro_input!(args as syn::LitStr).value();

    let lib_path = std::path::Path::new(&name);

    let dll = FileMap::open(&name).expect("Failed to open dll");

    let procs = get_proc_names(dll.as_ref()).expect("Failed to get proc names");

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
                std::arch::asm!(
                    "call [rdi]",
                    inout("rdi") &#sym_idents => _,
                );
            }
        )*
    }.into()
}
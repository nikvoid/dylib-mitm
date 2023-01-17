use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Comma, Unsafe};
use syn::{Ident, AttributeArgs, parse_macro_input, parse_quote, ItemFn, Expr, Visibility, TypeBareFn, BareFnArg, Signature};
use proc_macro2::Span;
use quote::{quote, format_ident};
use pelite::util::CStr;
use pelite::FileMap;
use proc_macro_error::{proc_macro_error, abort};

use darling::FromMeta;

#[derive(FromMeta)]
struct DylibMitmSpecifiedArgs {
    os: String,
    arch: String,
    proto_path: String,
    load_lib: Option<Expr>,
    manual_impls: Option<Punctuated<Ident, Comma>>,
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

fn get_symbol_marker(i: &Ident) -> Ident {
    let mut marker = format_ident!("{i}_export_implementation");
    marker.set_span(Span::mixed_site());
    marker
}

#[proc_macro_error(allow_not_macro)]
pub fn impl_dylib_mitm_specified(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let crate_name = Ident::new("dylib_mitm", Span::mixed_site());

    let args = parse_macro_input!(args as AttributeArgs);

    let args = match DylibMitmSpecifiedArgs::from_list(&args) {
        Ok(v) => v,
        Err(e) => { return proc_macro::TokenStream::from(e.write_errors()); }
    };

    let lib_path = std::path::Path::new(&args.proto_path);

    let dll = FileMap::open(&args.proto_path).expect("Failed to open dll");

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
    
    let (procs, manual_impls): (Vec<_>, _) =
    if let Some(manual_impls) = args.manual_impls {
        // Test if library has manually implemented exports
        if let Some(sym) = manual_impls.iter().find(|id| {
            let name = id.to_string();
            !procs.clone().any(|p| p == name) 
        }) {
            abort!(sym.span(), format!("Export `{sym}` not found in library"));
        }

        // Strip manually implemented exports
        let procs = procs.filter(
            |p| !manual_impls.iter().any(|m| p == &m.to_string())
        ).collect();
            
        (procs, manual_impls)
    } else {
        (procs.collect(), Punctuated::new())
    };

    let sym_idents: Vec<_> = procs.iter().map(|name| {
        Ident::new(&format!("__{name}"), Span::call_site())
    }).collect();
    
    let export_idents = procs.iter().map(|name| Ident::new(name, Span::call_site()));

    let mut lib_name = lib_path.file_name()
        .and_then(|n|
            n.to_str().map(|n| n.split('.').next().unwrap())
        ).expect("Failed to get dylib name").to_string();

    // TODO: Ensure that library name is valid rust ident
    lib_name = lib_name.replace('-', "_");

    let lib_struct = Ident::new(&lib_name, Span::call_site());

    let lib_ident = Ident::new(&format!("{lib_name}_LIB").to_uppercase(), Span::call_site());

    // Load lib at proto_path by default
    let proto_path = args.proto_path;
    let load_lib_expr: Expr = match args.load_lib {
        Some(expr) => expr,
        None => parse_quote!( #proto_path )
    };
    
    let marker_types = manual_impls.iter().map(get_symbol_marker);
    let manual_syms = manual_impls.iter().map(|i| format_ident!("__{i}"));

    quote! {
        #[allow(non_upper_case_globals)]
        static mut #lib_ident: Option<#lib_struct> = None;

        #(
            #[allow(non_upper_case_globals)]
            static mut #sym_idents: unsafe fn() = || { panic!("library was not loaded yet") };
        )*

        #[allow(non_camel_case_types)]
        struct #lib_struct(#crate_name::libloading::Library);

        impl #lib_struct {
            pub unsafe fn init() {
                let actual_lib_path: &str = #load_lib_expr;
                #lib_ident = Some(Self(#crate_name::libloading::Library::new(actual_lib_path)
                    .expect("Failed to load library")));
                let Some(#lib_struct(ref lib)) = #lib_ident else { unreachable!() };

                // A trick to force manual export implementation - marker type
                #( #manual_syms = *lib.get::<#marker_types>(stringify!(#manual_impls).as_bytes())
                    .expect("Failed to get symbol"); 
                )*
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

#[proc_macro_error(allow_not_macro)]
pub fn impl_manual_impl(
    _args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as ItemFn);

    if !matches!(item.vis, Visibility::Public(_)) {
        abort!(item.sig.fn_token.span(), "Manual export implementation must be public");
    }

    let sig = item.sig.clone();

    let marker_ty = TypeBareFn { 
        lifetimes: None,
        unsafety: Some(Unsafe::default()),
        abi: sig.abi,
        fn_token: sig.fn_token,
        paren_token: sig.paren_token,
        inputs: Punctuated::from_iter(sig.inputs.into_iter().map(|i| match i {
            syn::FnArg::Receiver(r) => abort!(r.span(), "Methods are not supported"),
            syn::FnArg::Typed(t) => BareFnArg { 
                attrs: t.attrs,
                name: None,
                ty: *t.ty 
            },
        })),
        variadic: sig.variadic,
        output: sig.output 
    };

    let marker = get_symbol_marker(&sig.ident);
    
    let import = format_ident!("__{}", sig.ident);

    // Unfortunately, closures can have only "Rust" abi, so we need function
    let mut stub_fn_ident = format_ident!("{}_stub_fn", sig.ident);
    stub_fn_ident.set_span(Span::mixed_site());

    let stub_fn_sig = Signature { 
        ident: stub_fn_ident.clone(),
        ..item.sig.clone() 
    };
    
    quote! {
        type #marker = #marker_ty;
        
        // This function serves as default value 
        #[allow(unused_variables, non_snake_case)]
        #stub_fn_sig { panic!("library was not loaded yet") }  
        
        #[allow(non_upper_case_globals)]
        static mut #import: #marker = #stub_fn_ident;

        #[no_mangle]
        #item
    }.into()
}
//! Provides process macros for `util` library.

#![deny(missing_docs)]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    parse_macro_input, spanned::Spanned, token::Extern, Abi, Error, FnArg, Ident, ItemFn, LitStr,
};

extern crate proc_macro;

macro_rules! error {
    ($tokens:expr, $msg:expr) => {
        Error::new($tokens.span(), $msg).to_compile_error()
    };
    ($tokens:expr, $fmt:expr, $($args:expr),*) => {
        error!($tokens, format!($fmt, $($args),*))
    };
}

/// Make a function meet the x64 interrupt calling convention. Then, pass a reference to
/// `util::interrupt::InterruptFrame` to a function.
///
/// Since some CPU exceptions pass an error code to handlers, can receive it to add an extra
/// [`u64`] parameter. Adding an extra argument causes UB when the CPU/software interruption does
/// not pass an error code.
///
/// # Example
///
/// - Default usage.
///
/// ```
/// #[interrupt_handler]
/// fn int_handler(frame: &InterruptFrame) {
///     // operations
/// }
/// ```
///
/// - With an error code.
///
/// ```
/// #[interrupt_handler]
/// fn int_handler(frame: &InterruptFrame, error_code: u64) {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn interrupt_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    // Collect errors to notify as many as possible at once.
    let mut errors = TokenStream2::new();

    // Accept no attributes.
    if !args.is_empty() {
        errors.append_all(error!(
            TokenStream2::from(args),
            "interrupt attribute does not accept any attributes."
        ));
    }

    let mut func = parse_macro_input!(input as ItemFn);

    // We accept one &InterruptFrame argument or &Interrupt Frame and an error code. Let the
    // compiler make sure that the types are matching. See the ABI checking section.
    let receive_error_code = match func.sig.inputs.len() {
        1 => false,
        2 => true,
        _ => {
            errors.append_all(error!(
                func.sig.inputs,
                "interrupt handler must receive one or two parameters"
            ));
            // dummy value
            false
        }
    };
    if let Some(asyncness) = func.sig.asyncness {
        errors.append_all(error!(asyncness, "interrupt handler cannot be async"));
    }
    if let Some(constness) = func.sig.constness {
        errors.append_all(error!(constness, "interrupt handler cannot be const"));
    }
    if !func.sig.generics.params.is_empty() {
        errors.append_all(error!(
            func.sig.generics,
            "interrupt handler cannot have generic parameters"
        ));
    }
    // Return all errors we can notify.
    if !errors.is_empty() {
        return errors.into_token_stream().into();
    }

    let old_ident = func.sig.ident.clone();
    let new_name = format!("_{}", old_ident);
    func.sig.ident = Ident::new(&new_name, func.sig.ident.span());

    // This is the actual interrupt handler. Save registers and call the handler after.
    // |    error code    |
    // |       RIP        |
    // |       CS         |
    // |      RFLAGS      |
    // |       RSP        |
    // |       SS         |
    #[rustfmt::skip]
    let caller_asm_code = format!(
        r#"
            .global {0}
            {0}:
                push rbp
                mov rbp, rsp

                # Adjust the RSP to 16-byte align.
                # It will align RSP properly because pushing 10, even words after this.
                and rsp, 0xfffffffffffffff0

                push rax
                push r11
                push r10
                push r9
                push r8
                push rdi
                push rsi
                push rdx
                push rcx
                push rbx
                cld
                {1}
                {2}
                call {3}
                pop rbx
                pop rcx
                pop rdx
                pop rsi
                pop rdi
                pop r8
                pop r9
                pop r10
                pop r11
                pop rax
                mov rsp, rbp
                pop rbp
                {4}
                iretq
        "#,
        old_ident,
        if receive_error_code { "lea rdi, [rbp + 0x10]" } else { "lea rdi, [rbp + 0x08]" },
        if receive_error_code { "mov rsi, [rbp + 0x08]" } else { "" },
        new_name,
        // Skip error code
        if receive_error_code { "add rsp, 0x08" } else { "" },
    );

    // Check the ABI and the parameters types of the function.
    let unsafety = &func.sig.unsafety;
    let inputs_types = func.sig.inputs.iter().map(|arg| match arg {
        FnArg::Receiver(arg) => quote!(#arg),
        FnArg::Typed(arg) => {
            let ty = &arg.ty;
            quote!(#ty)
        }
    });
    let output_type = &func.sig.output;
    let func_ident = &func.sig.ident;

    // Accept non-specified and compatible with sysv64 ABI. (On Unix system, C and sysv64 ABI are
    // the same.)
    // We add sysv64 ABI when not specified, then reqest ABI checking to the compiler. Check the
    // end of the function.
    if func.sig.abi.is_none() {
        func.sig.abi = Some(Abi {
            extern_token: Extern {
                span: func.sig.span(),
            },
            name: Some(LitStr::new("sysv64", func.sig.span())),
        })
    }
    let abi = func.sig.abi.as_ref().unwrap();

    // Use the compiler functionality that checks types of LHS and RHS of variable declaration.
    let func_type_check = if receive_error_code {
        quote_spanned! { func.span() =>
        const _:
            #unsafety extern "sysv64" fn(&::util::interrupt::InterruptFrame, u64) #output_type =
            #func_ident as #unsafety #abi fn(#(#inputs_types),*) #output_type;
        }
    } else {
        quote_spanned! { func.span() =>
        const _:
            #unsafety extern "sysv64" fn(&::util::interrupt::InterruptFrame) #output_type =
            #func_ident as #unsafety #abi fn(#(#inputs_types),*) #output_type;
        }
    };

    let ret = quote! {
        #func_type_check

        extern "sysv64" {
            fn #old_ident();
        }
        ::core::arch::global_asm!(#caller_asm_code);

        #[no_mangle]
        #func
    };

    ret.into()
}

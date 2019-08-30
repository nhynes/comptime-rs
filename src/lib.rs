//! Lightweight compile-time expression evaluation.
//! This crate is inspired by [Zig's `comptime`](https://ziglang.org/documentation/master/#comptime).
//!
//! The expression returned by the contents of the comptime macro invocation will be parsed as
//! Rust source code and inserted back into the call site.
//!
//! ### Example
//!
//! ```
//! #![feature(proc_macro_hygiene)]
//! fn main() {
//!     println!(concat!(
//!         "This program was compiled at ",
//!         comptime::comptime! {
//!             chrono::Utc::now()
//!         },
//!         "."
//!     )); // "This program was compiled at 2019-08-30 03:52:58.496747469 UTC."
//! }
//! ```

extern crate proc_macro;

use std::{path::Path, process::Command};

use proc_macro::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::parse::{Parse, ParseStream};

macro_rules! err {
    ($fstr:literal$(,)? $( $arg:expr ),*) => {{
        let compile_error = format!($fstr, $($arg),*);
        return TokenStream::from(quote!(compile_error!(#compile_error)));
    }};
}

struct BlockInner {
    stmts: Vec<syn::Stmt>,
}

impl Parse for BlockInner {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            stmts: syn::Block::parse_within(input)?,
        })
    }
}

impl ToTokens for BlockInner {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append_all(self.stmts.iter());
    }
}

#[proc_macro]
pub fn comptime(input: TokenStream) -> TokenStream {
    let args: Vec<_> = std::env::args().collect();
    let get_arg = |arg| {
        args.iter()
            .position(|a| a == arg)
            .and_then(|p| args.get(p + 1))
    };

    let is_check = args
        .iter()
        .find(|a| a.starts_with("--emit"))
        .map(|emit| !emit.contains("link"))
        .unwrap_or_default();

    let comptime_program = if !is_check {
        syn::parse_macro_input!(input as BlockInner)
    } else {
        syn::parse_quote!("")
    };

    let out_dir = match get_arg("--out-dir") {
        Some(out_dir) => out_dir,
        None => {
            err!("comptime failed: could not determine rustc out dir.");
        }
    };

    let comptime_rs = Path::new(out_dir).join("comptime.rs");
    std::fs::write(
        &comptime_rs,
        quote! {
            fn main() {
                print!("{}", { #comptime_program });
            }
        }
        .to_string(),
    )
    .expect("could not write comptime.rs");
    Command::new("rustfmt").arg(&comptime_rs).output().ok();

    let mut rustc_args = filter_rustc_args(&args);
    rustc_args.push("--crate-name".to_string());
    rustc_args.push("comptime_bin".to_string());
    rustc_args.push("--crate-type".to_string());
    rustc_args.push("bin".to_string());
    rustc_args.push(comptime_rs.to_str().unwrap().to_string());

    let compile_output = Command::new("rustc")
        .args(&rustc_args)
        .output()
        .expect("could not invoke rustc");
    if !compile_output.status.success() {
        err!(
            "could not compile comptime expr:\n\n{}\n",
            String::from_utf8(compile_output.stderr).unwrap()
        );
    }

    let extra_filename = args
        .iter()
        .find(|a| a.starts_with("extra-filename="))
        .map(|ef| ef.split('=').nth(1).unwrap())
        .unwrap_or_default();
    let comptime_bin = Path::new(out_dir).join(format!("comptime_bin{}", extra_filename));

    let comptime_output = Command::new(comptime_bin)
        .output()
        .expect("could not invoke comptime_bin");

    if !comptime_output.status.success() {
        err!(
            "could not run comptime expr:\n\n{}\n",
            String::from_utf8(comptime_output.stderr).unwrap()
        );
    }

    let comptime_expr_str = match String::from_utf8(comptime_output.stdout) {
        Ok(output) => output,
        Err(_) => err!("comptime expr output was not utf8"),
    };
    let comptime_expr: syn::Expr = match syn::parse_str(&comptime_expr_str) {
        Ok(expr) => expr,
        Err(_) => syn::ExprLit {
            attrs: Vec::new(),
            lit: syn::LitStr::new(&comptime_expr_str, proc_macro2::Span::call_site()).into(),
        }
        .into(),
    };

    TokenStream::from(ToTokens::to_token_stream(&comptime_expr))
}

/// Returns the rustc args needed to build the comptime executable.
fn filter_rustc_args(args: &[String]) -> Vec<String> {
    let mut rustc_args = Vec::with_capacity(args.len());
    let mut skip = true; // skip the invoked program
    for arg in args {
        if skip {
            skip = false;
            continue;
        }
        if arg == "--crate-type" || arg == "--crate-name" {
            skip = true;
        } else if arg.ends_with(".rs") || arg == "--test" || arg == "rustc" {
            continue;
        } else {
            rustc_args.push(arg.clone());
        }
    }
    rustc_args
}

//! Lightweight compile-time expression evaluation.
//! This crate is inspired by [Zig's `comptime`](https://ziglang.org/documentation/master/#comptime).
//!
//! The expression returned by the contents of the comptime macro invocation will be parsed as
//! Rust source code and inserted back into the call site.
//!
//! **tl;dr:** `comptime!` gives you no-context anonynmous proc macros.
//!
//! ### Example
//!
//! ``` compile_fail
//! fn main() {
//!     println!(concat!(
//!         "The program was compiled on ",
//!         comptime::comptime! {
//!             chrono::Utc::now().format("%Y-%m-%d").to_string()
//!         },
//!         "."
//!     )); // The program was compiled on 2019-08-30.
//! }
//! ```
//!
//! ### Limitations
//!
//! Unlike Zig, `comptime!` does not have access to the scope in which it is invoked.
//! The code in `comptime!` is run as its own script. Though, technically, you could
//! interpolate static values using `quote!`.
//!
//! Also, `comptime!` requires you to run `cargo build` at least once before `cargo (clippy|check)`
//! will work since `comptime!` does not compile dependencies.
//!
//! Finally, using this macro in doctests may fail with strange errors for no good reason. This is
//! because output directory detection is imperfect and sometimes breaks. You have been warned.

extern crate proc_macro;
use std::{
    collections::{
        hash_map::{DefaultHasher, Entry},
        HashMap,
    },
    hash::{Hash, Hasher},
    path::Path,
    process::Command,
};
mod comptime_impl;
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

#[proc_macro_attribute]
pub fn comptime_fn(args: TokenStream, item: TokenStream) -> TokenStream {
    comptime_impl::comptime_impl(args, item)
}

#[proc_macro]
pub fn comptime(input: TokenStream) -> TokenStream {
    let args: Vec<_> = std::env::args().collect();
    let get_arg = |arg| {
        args.iter()
            .position(|a| a == arg)
            .and_then(|p| args.get(p + 1))
    };

    let comptime_program = syn::parse_macro_input!(input as BlockInner);

    let out_dir = match get_arg("--out-dir") {
        Some(out_dir) => Path::new(out_dir),
        None => {
            err!("comptime failed: could not determine rustc out dir.");
        }
    };

    let comptime_program_str = comptime_program.to_token_stream().to_string();
    let mut hasher = DefaultHasher::new();
    comptime_program_str.hash(&mut hasher);
    let comptime_disambiguator = hasher.finish();

    let comptime_rs = out_dir.join(format!("comptime-{}.rs", comptime_disambiguator));
    std::fs::write(
        &comptime_rs,
        format!(
            r#"fn main() {{
                    let comptime_output = {{ {} }};
                    print!("{{}}", quote::quote!(#comptime_output));
                }}"#,
            comptime_program_str
        ),
    )
    .expect("could not write comptime.rs");
    Command::new("rustfmt").arg(&comptime_rs).output().ok();

    let mut rustc_args = filter_rustc_args(&args);
    rustc_args.push("--crate-name".to_string());
    rustc_args.push("comptime_bin".to_string());
    rustc_args.push("--crate-type".to_string());
    rustc_args.push("bin".to_string());
    rustc_args.push("--emit=dep-info,link".to_string());
    rustc_args.append(&mut merge_externs(&out_dir, &args));
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
    let comptime_bin = out_dir.join(format!("comptime_bin{}", extra_filename));

    let comptime_output = Command::new(&comptime_bin)
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

    std::fs::remove_file(comptime_rs).ok();
    std::fs::remove_file(comptime_bin).ok();

    TokenStream::from(comptime_expr.to_token_stream())
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
        if arg == "--crate-type" || arg == "--crate-name" || arg == "--extern" {
            skip = true;
        } else if arg.ends_with(".rs")
            || arg == "--test"
            || arg == "rustc"
            || arg.starts_with("--emit")
        {
            continue;
        } else {
            rustc_args.push(arg.clone());
        }
    }
    rustc_args
}

fn merge_externs(deps_dir: &Path, args: &[String]) -> Vec<String> {
    let mut cargo_rlibs = HashMap::new(); // libfoo -> /path/to/libfoo-12345.rlib
    let mut next_is_extern = false;
    for arg in args {
        if next_is_extern {
            let mut libname_path = arg.split('=');
            let lib_name = libname_path.next().unwrap(); // libfoo
            let path = Path::new(libname_path.next().unwrap());
            if path.extension().unwrap() == "rlib" {
                cargo_rlibs.insert(lib_name.to_string(), path.to_path_buf());
            }
        }
        next_is_extern = arg == "--extern";
    }

    let mut dep_dirents: Vec<_> = std::fs::read_dir(deps_dir)
        .unwrap()
        .filter_map(|de| {
            let de = de.unwrap();
            let p = de.path();
            let fname = p.file_name().unwrap().to_str().unwrap();
            if fname.starts_with("lib") && fname.ends_with(".rlib") {
                Some(de)
            } else {
                None
            }
        })
        .collect();
    dep_dirents.sort_by_key(|de| std::cmp::Reverse(de.metadata().and_then(|m| m.created()).ok()));

    for dirent in dep_dirents {
        let path = dirent.path();
        let fname = path.file_name().unwrap().to_str().unwrap();
        if !fname.ends_with(".rlib") {
            continue;
        }
        let lib_name = fname.rsplitn(2, '-').nth(1).unwrap().to_string();
        // ^ reverse "libfoo-disambiguator" then split off the disambiguator
        if let Entry::Vacant(ve) = cargo_rlibs.entry(lib_name) {
            ve.insert(path);
        }
    }

    let mut merged_externs = Vec::with_capacity(cargo_rlibs.len() * 2);
    for (lib_name, path) in cargo_rlibs.iter() {
        merged_externs.push("--extern".to_string());
        merged_externs.push(format!("{}={}", &lib_name["lib".len()..], path.display()));
    }

    merged_externs
}

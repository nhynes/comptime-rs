Lightweight compile-time expression evaluation.
This crate is inspired by [Zig's `comptime`](https://ziglang.org/documentation/master/#comptime).

The expression returned by the contents of the comptime macro invocation will be parsed as
Rust source code and inserted back into the call site.

**tl;dr:** `comptime!` gives you no-context anonynmous proc macros.

### Example

## proc-macro

```rust
#![feature(proc_macro_hygiene)]
fn main() {
    println!(concat!(
        "The program was compiled on ",
        comptime::comptime! {
            chrono::Utc::now().format("%Y-%m-%d").to_string()
        },
        "."
    )); // The program was compiled on 2024-05-22.
}
```

## Attribute macro

```rust
fn main() {
    println!("{}", at_comptime()); // The program was compiled on 2024-05-22.
}
#[comptime::comptime_fn]
fn at_comptime() -> &'static str {
    format!(concat!(
        "The program was compiled on ",
        comptime::comptime! {
            chrono::Utc::now().format("%Y-%m-%d").to_string()
        },
        "."
    ))
}
```

### Limitations

Unlike the real `comptime`, `comptime!` does not have access to the scope in which it is invoked.
The code in `comptime!` is run as its own script.
Though, technically, you could interpolate static values using `quote!`.

Also, `comptime!` requires you to run `cargo build` at least once before `cargo (clippy|check)`
will work since `comptime!` does not compile dependencies.

Strings generated with comptime_fn will be represented as `&'static str` as it is known at compile time, to fix this, you need to simply run `String::from(comptime_fn())` or `comptime_fn().to_string()`.

Due to how the comptime macro works, writing to stdout will almost definitely cause comptime-rs to faile to build.

The comptime_fn attribute macro still makes the function call to the compile time function, but all calculations inside that function are performed at compile time. e.g.

```rust
#[comptime::comptime_fn]
fn costly_calculation() -> i32 {
    2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 // Any calculations
}
```

will be turned into

```rust
#[comptime::comptime_fn]
fn costly_calculation() -> i32 {
    362880
}
```

### Contributing

Please do!
Ideally, `rustc` would also have (real) `comptime` which would have access to type information and other static values.
In the meantime, this should be a nice way to approximate and experiment with such functionality.

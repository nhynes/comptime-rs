Lightweight compile-time expression evaluation.
This crate is inspired by [Zig's `comptime`](https://ziglang.org/documentation/master/#comptime).

The expression returned by the contents of the comptime macro invocation will be parsed as
Rust source code and inserted back into the call site.

**tl;dr:** `comptime!` gives you anonynmous proc macros.

### Example

```rust
#![feature(proc_macro_hygiene)]
fn main() {
    println!(concat!(
        "The program was compiled on ",
        comptime::comptime! {
            chrono::Utc::now().format("%Y-%m-%d").to_string()
        },
        "."
    )); // The program was compiled on 2019-08-30.
}
```

### Limitations

Unlike the real `comptime`, `comptime!` does not have access to the scope in which it is invoked.
The code in `comptime!` is run as its own script.
Though, technically, you could interpolate static values using `quote!`.

Also, `comptime!` requires you to run `cargo build` at least once before `cargo (clippy|check)`
will work since `comptime!` does not compile dependencies.

### Contributing

Please do!
Ideally, `rustc` would also have (real) `comptime` which would have access to type information and other static values.
In the meantime, this should be a nice way to approximate and experiment with such functionality.

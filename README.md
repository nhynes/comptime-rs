Lightweight compile-time expression evaluation.
This crate is inspired by [Zig's `comptime`](https://ziglang.org/documentation/master/#comptime).

The expression returned by the contents of the comptime macro invocation will be parsed as
Rust source code and inserted back into the call site.

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

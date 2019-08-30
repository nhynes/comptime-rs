Lightweight compile-time expression evaluation.
This crate is inspired by [Zig's `comptime`](https://ziglang.org/documentation/master/#comptime).

The expression returned by the contents of the comptime macro invocation will be parsed as
Rust source code and inserted back into the call site.

### Example

```rust
#![feature(proc_macro_hygiene)]
fn main() {
    println!(concat!(
        "This program was compiled at ",
        comptime::comptime! {
            chrono::Utc::now()
        },
        "."
    )); // "This program was compiled at 2019-08-30 03:52:58.496747469 UTC."
}
```

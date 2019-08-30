#![feature(proc_macro_hygiene)]

fn main() {
    println!(concat!(
        "This program was compiled at ",
        comptime::comptime! {
            chrono::Utc::now()
        },
        "."
    ));
}

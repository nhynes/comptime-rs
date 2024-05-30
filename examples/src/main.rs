fn main() {
    println!(concat!(
        "The program was compiled on ",
        comptime::comptime! {
            chrono::Utc::now().format("%Y-%m-%d").to_string()
        },
        "."
    ));
    // attribute macro comptime functions cannot be used as "literals"
    // as there is still a function call being made, even if though it
    // just immediately returns an `&'static str`.
    println!("This program was compiled on {}", test());
}

#[comptime::comptime_fn]
fn test() -> &'static str {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

#![feature(proc_macro_hygiene)]
#![cfg(test)]

#[macro_use]
extern crate comptime;

#[test]
fn test_basic() {
    assert_eq!(
        concat!("u32 is ", comptime!(std::mem::size_of::<u32>()), " bytes"),
        "u32 is 4 bytes"
    );
}

#[test]
fn test_inner_mac() {
    assert_eq!(comptime!(stringify!(4)), "4");
}

#[test]
fn test_inner_crate() {
    assert_eq!(
        comptime! {
            use rand::{SeedableRng, RngCore};
            rand::rngs::StdRng::seed_from_u64(42u64).next_u64()
        },
        9_482_535_800_248_027_256u64
    );
}

#![cfg(all(feature = "derive", any(feature = "default", feature = "std")))]

#[test]
fn compile_fail() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/compile_fail/*.rs");
}

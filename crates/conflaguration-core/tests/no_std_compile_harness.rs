use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn compile_fixture(directory: &Path, name: &str, fixture: &str) -> Output {
    let source = format!("{}\n{fixture}", include_str!("../src/lib.rs"));
    let input = directory.join(format!("{name}.rs"));
    fs::write(directory.join("resolve.rs"), include_str!("../src/resolve.rs")).expect("resolve module is written beside the temporary fixture");
    fs::write(&input, source).expect("fixture source is written to a temporary directory");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    Command::new(rustc)
        .args(["--edition=2024", "--crate-type=lib", "--target=x86_64-unknown-none", "--emit=metadata", "--out-dir"])
        .arg(directory)
        .arg(&input)
        .output()
        .expect("rustc can compile the fixture")
}

#[test]
fn core_only_and_std_rejected() {
    let directory = tempfile::tempdir().expect("compile fixtures use a temporary directory");
    let incumbent = compile_fixture(directory.path(), "core_only", include_str!("fixtures/core_only.rs"));
    assert!(incumbent.status.success(), "core-only fixture must compile: {}", String::from_utf8_lossy(&incumbent.stderr));
    let control = compile_fixture(directory.path(), "std_env", include_str!("fixtures/std_env.rs"));
    assert!(!control.status.success(), "std fixture must fail");
    let error = String::from_utf8_lossy(&control.stderr);
    assert!(error.contains("std"), "rejection must identify std: {error}");
    println!("{{\"core_fixture_accepted\":1,\"std_fixture_rejected\":1}}");
    println!("std fixture compiler stderr:\n{error}");
}

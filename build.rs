
fn main() -> miette::Result<()> {
    let path = std::path::PathBuf::from("src");
    let mut b = autocxx_build::Builder::new("src/main.rs",
     [&path])
        .extra_clang_args(&["-std=c++17"])
        .build()?;
    b.compiler("clang++").flag("-std=c++17")
        .file("src/cc/polygon.cc")
        .compile("tarantula");

    println!("cargo:rustc-link-lib=s2");
    println!("cargo:rustc-link-lib=absl_log_internal_message");
    println!("cargo:rustc-link-lib=absl_log_internal_check_op");
    println!("cargo:rustc-link-lib=absl_raw_logging_internal");
    println!("cargo:rustc-link-lib=absl_hash");
    println!("cargo:rustc-link-lib=absl_raw_hash_set");
 
    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=tests/test.rs");
    println!("cargo:rerun-if-changed=src/cc/polygon.cc");

    tonic_build::compile_protos("proto/service.proto").unwrap();

    Ok(())
}
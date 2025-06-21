
fn main() -> miette::Result<()> {
    let path = std::path::PathBuf::from("src");
    let mut b = autocxx_build::Builder::new("src/main.rs",
     [&path]).build()?;
    b.flag_if_supported("-std=c++17")
        .file("src/cc/polygon.cc")
        .compile("tarantula");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=tests/test.rs");
    println!("cargo:rerun-if-changed=src/cc/polygon.cc");

    tonic_build::compile_protos("proto/service.proto").unwrap();

    Ok(())
}
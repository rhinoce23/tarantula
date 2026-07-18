
fn main() -> miette::Result<()> {
    let docs_rs = std::env::var_os("DOCS_RS").is_some() || std::env::var_os("CARGO_CFG_DOCSRS").is_some();
    if docs_rs {
        println!("cargo:rerun-if-changed=src/main.rs");
        println!("cargo:rerun-if-changed=tests/test.rs");
        println!("cargo:rerun-if-changed=src/cc/polygon.cc");
        return Ok(());
    }

    let path = std::path::PathBuf::from("src");
    let mut b = autocxx_build::Builder::new("src/lib.rs", [&path])
        .extra_clang_args(&["-std=c++17"])
        .build()?;
    b.compiler("clang++")
        .flag("-std=c++17")
        .file("src/cc/polygon.cc")
        .compile("tarantula");

    println!("cargo:rustc-link-lib=s2");

    for dir in ["/opt/homebrew/lib", "/opt/homebrew/opt/abseil/lib", "/opt/homebrew/opt/openssl@3/lib", "/usr/local/lib", "/usr/local/opt/abseil/lib"] {
        if std::path::Path::new(dir).exists() {
            println!("cargo:rustc-link-search=native={dir}");
        }
    }

    for dir in ["/opt/homebrew/opt/abseil/lib", "/usr/local/opt/abseil/lib"] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut absl_libs = entries
                .flatten()
                .filter_map(|entry| {
                    let path = entry.path();
                    let name = path.file_name()?.to_str()?;
                    let trimmed = name.strip_prefix("lib")?.strip_suffix(".dylib")?;
                    trimmed.starts_with("absl_").then(|| trimmed.to_string())
                })
                .collect::<Vec<_>>();
            absl_libs.sort();
            absl_libs.dedup();
            for lib in absl_libs {
                println!("cargo:rustc-link-lib=dylib={lib}");
            }
        }
    }

    println!("cargo:rustc-link-arg=-Wl,-rpath,/opt/homebrew/opt/abseil/lib");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/opt/homebrew/opt/openssl@3/lib");

    println!("cargo:rerun-if-changed=src/main.rs");
    println!("cargo:rerun-if-changed=tests/test.rs");
    println!("cargo:rerun-if-changed=src/cc/polygon.cc");

    tonic_build::compile_protos("proto/service.proto").unwrap();

    Ok(())
}
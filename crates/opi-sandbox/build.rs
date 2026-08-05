fn main() {
    let target = std::env::var("TARGET").expect("Cargo supplies TARGET to build scripts");
    println!("cargo:rustc-env=OPI_SANDBOX_BUILD_TARGET={target}");
}

fn main() {
    if let Ok(bundled) = std::env::var("Z3RM_BUNDLE") {
        println!("cargo:rustc-env=Z3RM_BUNDLE={}", bundled);
    }
}

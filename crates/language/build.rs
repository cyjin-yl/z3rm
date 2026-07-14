fn main() {
    if let Ok(bundled) = std::env::var("ZERMINAL_BUNDLE") {
        println!("cargo:rustc-env=ZERMINAL_BUNDLE={}", bundled);
    }
}

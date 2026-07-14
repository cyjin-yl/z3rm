fn main() {
    let cargo_toml =
        std::fs::read_to_string("../zerminal/Cargo.toml").expect("Failed to read crates/zerminal/Cargo.toml");
    let version = cargo_toml
        .lines()
        .find(|line| line.starts_with("version = "))
        .expect("Version not found in crates/zerminal/Cargo.toml")
        .split('=')
        .nth(1)
        .expect("Invalid version format")
        .trim()
        .trim_matches('"');
    println!("cargo:rerun-if-changed=../zerminal/Cargo.toml");
    println!("cargo:rustc-env=ZED_PKG_VERSION={}", version);
}

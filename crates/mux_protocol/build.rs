// §9 mux_protocol 构建脚本：使用 prost-build 编译 proto/mux.proto。
fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(&["proto/mux.proto"], &["proto/"])?;
    Ok(())
}

//! # mux_protocol
//!
//! z3rm 客户端与 mux_server 之间的 prost/protobuf 有线协议。
//! 协议版本化（§3.10），基于长度前缀的二进制帧（§9），
//! 覆盖会话生命周期、Pane 生命周期、网格同步、滚动缓冲、文件读取、
//! 剪贴板中继以及扩展 Chrome RPC（§16）。

use prost::Message;

// §9 由 prost-build 生成的 protobuf 类型，命名空间为 z3rm.mux。
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/z3rm.mux.rs"));
}

pub use proto::*;

// §3.10 当前协议版本：major 用于破坏性变更，minor 用于新增字段。
pub const PROTOCOL_VERSION: proto::ProtocolVersion = proto::ProtocolVersion {
    major: 1,
    minor: 0,
};

// §9 将 Envelope 编码为长度前缀二进制帧：| len: u32 | protobuf bytes |。
/// Frame a message as length-prefixed binary.
pub fn frame(msg: &Envelope) -> Result<Vec<u8>, prost::EncodeError> {
    let mut buf = Vec::with_capacity(msg.encoded_len() + 4);
    (msg.encoded_len() as u32).encode(&mut buf)?;
    msg.encode(&mut buf)?;
    Ok(buf)
}

// §9 从长度前缀二进制帧解码 Envelope，返回 (消息, 已消费字节数)。
/// Decode a framed message. Returns (message, bytes_consumed).
pub fn unframe(buf: &[u8]) -> Result<(Envelope, usize), prost::DecodeError> {
    if buf.len() < 4 {
        return Err(prost::DecodeError::new("buffer too short for frame header"));
    }
    let len = u32::decode(&buf[..4])? as usize;
    if buf.len() < 4 + len {
        return Err(prost::DecodeError::new("incomplete frame"));
    }
    let msg = Envelope::decode(&buf[4..4 + len])?;
    Ok((msg, 4 + len))
}

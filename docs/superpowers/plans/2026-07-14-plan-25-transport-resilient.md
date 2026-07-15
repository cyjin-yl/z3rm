# Plan 25: transport_resilient — UDP Resilient Transport

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Implement the mosh-inspired UDP resilient transport: per-packet AEAD, stateless roaming, RTT estimation, frame-rate control, heartbeat. This is a transport variant for remote connections with flaky networks. Used alongside SSH tunnel transport — SSH is reliable, UDP is resilient.

**Architecture:** `UdpResilientTransport` implements the same framed-binary I/O interface as `LocalSocketTransport` and `SshTunnelTransport`. It carries mux_protocol prost Envelopes over UDP with per-packet AEAD encryption. Stateless roaming: server latches onto packet source address. Frame-rate control for grid updates.

**Dependencies:** `mux_protocol`, `tokio`, `aes-gcm`, `tokio-util`.

---

### Task 1: Create crate skeleton

**Files:**
- Create: `crates/transport_resilient/Cargo.toml`
- Create: `crates/transport_resilient/src/transport_resilient.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "transport_resilient"
version = "0.1.0"
edition = "2024"
publish = false
license = "Apache-2.0"

[lib]
path = "src/transport_resilient.rs"

[dependencies]
mux_protocol = { workspace = true }
tokio = { workspace = true, features = ["full"] }
aes-gcm = "0.10"
tokio-util = { workspace = true, features = ["codec"] }
prost = { workspace = true }
anyhow = { workspace = true }
parking_lot = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 2: Add to workspace Cargo.toml**

---

### Task 2: Per-packet AEAD

**Files:**
- Create: `crates/transport_resilient/src/crypto.rs`

- [ ] **Step 1: Implement AES-256-GCM per-packet encryption**

```rust
pub struct PacketCodec {
    key: [u8; 32],
    send_nonce: AtomicU64,
    recv_window: PacketWindow,  // replay protection
}

impl PacketCodec {
    pub fn encrypt(&self, plaintext: &[u8]) -> Vec<u8> {
        let nonce = self.send_nonce.fetch_add(1, Ordering::SeqCst);
        // nonce = 12 bytes: 4-byte prefix + 8-byte counter
        let nonce_bytes = nonce.to_be_bytes();
        let nonce_full = [&self.nonce_prefix[..], &nonce_bytes[..]][..].concat();
        let cipher = aes_gcm::Aes256Gcm::new_from_slice(&self.key).unwrap();
        let ciphertext = cipher.encrypt(&Nonce::from_slice(&nonce_full), plaintext).unwrap();
        // packet = [nonce_bytes (8)] + [ciphertext]
        [nonce_bytes.as_ref(), &ciphertext[..]].concat()
    }
    
    pub fn decrypt(&self, packet: &[u8]) -> Result<Vec<u8>> {
        // Extract nonce, check replay window, decrypt
        todo!("implement decrypt with replay window check")
    }
}
```

- [ ] **Step 2: Implement replay protection window**

Track received nonce values. Reject duplicates or out-of-order packets older than window size (default 64).

---

### Task 3: Stateless roaming

**Files:**
- Create: `crates/transport_resilient/src/transport.rs`

- [ ] **Step 1: Server-side source address latching**

```rust
pub struct UdpServer {
    socket: UdpSocket,
    client_addr: parking_lot::Mutex<SocketAddr>,  // updated on every valid packet
}

impl UdpServer {
    async fn recv(&self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; 65536];
        let (len, addr) = self.socket.recv_from(&mut buf).await?;
        // Update client address (roaming)
        *self.client_addr.lock() = addr;
        // Decrypt
        self.codec.decrypt(&buf[..len])
    }
    
    async fn send(&self, data: &[u8]) -> Result<()> {
        let addr = *self.client_addr.lock();
        let encrypted = self.codec.encrypt(data);
        self.socket.send_to(&encrypted, addr).await?;
        Ok(())
    }
}
```

When client changes IP (roaming), server automatically retargets to the new source address of the next valid authenticated packet. No reconnection handshake.

---

### Task 4: RTT estimation + frame-rate control

**Files:**
- Create: `crates/transport_resilient/src/rtt.rs`

- [ ] **Step 1: Implement SRTT/RTTVAR estimation**

TCP's algorithm with modifications (from mosh paper):
- Unique seq per datagram for disambiguation
- Timestamp-reply adjusted by time-since-receipt
- RTO floor: 50ms (not TCP's 1s)

```rust
pub struct RttEstimator {
    srtt: Duration,
    rttvar: Duration,
    rto: Duration,  // min 50ms, max 1000ms
}
```

- [ ] **Step 2: Implement frame-rate control**

Send interval = `clamp(SRTT / 2, 20ms, 250ms)`. This controls how frequently the server pushes grid updates to the client.

- [ ] **Step 3: Implement heartbeat**

ACK_INTERVAL = 3000ms (heartbeat). If no data for 3s, send heartbeat packet. Server detaches after SERVER_ASSOCIATION_TIMEOUT = 40s of silence.

---

### Task 5: UdpResilientTransport implementation

**Files:**
- Create: `crates/transport_resilient/src/transport_resilient.rs`

- [ ] **Step 1: Implement framed I/O over UDP**

mux_protocol Envelopes are serialized, encrypted, and sent as UDP datagrams. Large messages are fragmented (MTU = 1280 bytes).

- [ ] **Step 2: Implement client-side transport**

```rust
pub struct UdpResilientTransport {
    socket: UdpSocket,
    server_addr: SocketAddr,
    codec: PacketCodec,
    rtt: RttEstimator,
}

impl UdpResilientTransport {
    pub async fn connect(server_addr: SocketAddr, session_key: [u8; 32]) -> Result<Self>;
    pub async fn send(&self, msg: &Envelope) -> Result<()>;
    pub async fn recv(&self) -> Result<Envelope>;
}
```

- [ ] **Step 3: Integrate into MuxTransport enum**

Add `Udp(UdpResilientTransport)` variant to `MuxTransport`. Transport selection at connect time: SSH bootstrap exchanges session key → if RTT ≥ threshold, switch to UDP resilient.

---

### Task 6: Bootstrap (SSH key exchange)

- [ ] **Step 1: SSH exec to exchange session key, then switch to UDP**

```
1. SSH to remote host
2. Spawn remote mux_server --udp
3. Server generates AES-256 key, binds UDP port
4. Server prints "Z3RM_UDP <port> <base64-key>" over SSH stdout
5. SSH channel closes
6. Client opens UDP to server:port with received key
```

---

### Task 7: Tests

- [ ] **Step 1: AEAD encrypt/decrypt round-trip test**

- [ ] **Step 2: Replay window test (reject duplicate nonce)**

- [ ] **Step 3: Stateless roaming test (change source address mid-session)**

- [ ] **Step 4: RTT estimation accuracy test (simulated latency)**

- [ ] **Step 5: Frame-rate control test (verify send interval adapts to RTT)**

- [ ] **Step 6: Network fault injection: packet loss test**

Simulate 30% packet loss. Verify session survives, grid eventually consistent.

- [ ] **Step 7: Commit**

```bash
git add crates/transport_resilient Cargo.toml
git commit -m "Add transport_resilient: UDP AEAD, stateless roaming, RTT estimation, frame-rate control"
```

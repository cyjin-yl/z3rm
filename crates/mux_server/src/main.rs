// §3.1 z3rm-server daemon 入口点。
// 绑定本地 socket，接受连接，服务 mux protocol RPC。

use anyhow::Result;
use mux_server::run;

fn main() -> Result<()> {
    run()
}

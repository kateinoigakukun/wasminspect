use std::net::SocketAddr;
use std::str::FromStr;

use env_logger;
use structopt::StructOpt;
use wasminspect_debugger_server;

#[derive(StructOpt)]
struct Opts {
    /// The wasm binary file
    #[structopt(default_value = "127.0.0.1:4000")]
    listen_addr: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("warn"));

    let opts = Opts::from_args();
    let addr = SocketAddr::from_str(&opts.listen_addr)?;
    wasminspect_debugger_server::start(addr).await;
    Ok(())
}

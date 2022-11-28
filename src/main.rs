use anyhow::Result;
use clap::Parser;
use node::start_node;

use crate::args::Args;

mod args;
mod node;
mod raft_network;
mod store;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    start_node(args.id, args.bind_addr).await
}

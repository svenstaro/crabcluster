use anyhow::Result;
use clap::Parser;
use node::start_node;
use tracing_subscriber::{filter::LevelFilter, prelude::*, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;

use crate::args::Args;

mod args;
mod node;
mod raft_network;
mod store;

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .parse("")?;

    let layer = HierarchicalLayer::default()
        .with_writer(std::io::stdout)
        .with_indent_lines(true)
        .with_indent_amount(2)
        .with_thread_names(false)
        .with_thread_ids(false)
        .with_verbose_exit(false)
        .with_verbose_entry(false)
        .with_targets(true)
        .with_filter(env_filter);
    let subscriber = Registry::default().with(layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let args = Args::parse();
    start_node(args.id, args.bind_addr).await
}

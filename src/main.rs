use anyhow::Result;
use clap::Parser;
use node::start_node;
use tracing_subscriber::{filter::LevelFilter, prelude::*, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;
use uuid::Uuid;

use crate::args::Args;

mod args;
mod network;
mod node;
mod raft_network;
mod store;

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .parse("info,crabcluster=debug,openraft=info,tower_http=debug")?;

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

    // TODO: Eventually store and restore this generated ID from disk.
    let node_id = Uuid::new_v4();

    // Use the default podman socket for this user.
    let podman_socket_path = directories::BaseDirs::new()
        .expect("Didn't find base dirs")
        .runtime_dir()
        .expect("No runtime dir found")
        .join("podman/podman.sock");

    dbg!(&podman_socket_path);
    let podman = podman_api::Podman::unix(podman_socket_path);
    dbg!(podman.info().await?);

    start_node(node_id, args.bind_addr).await
}

use anyhow::Result;

mod node;
mod raft_network;
mod store;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");
    Ok(())
}

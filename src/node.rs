use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use openraft::{BasicNode, Config, Raft};
use uuid::Uuid;

use crate::raft_network::RaftNetworkConfig;
use crate::store::{RaftRequest, RaftResponse, RaftStore};

pub type NodeId = Uuid;

openraft::declare_raft_types!(
    /// Declare the type configuration for K/V store.
    pub RaftTypeConfig: D = RaftRequest, R = RaftResponse, NodeId = NodeId, Node = BasicNode
);

pub type RaftConfig = Raft<RaftTypeConfig, RaftNetworkConfig, Arc<RaftStore>>;

// Representation of an application state. This struct can be shared around to share
// instances of raft, store and more.
pub struct RaftApp {
    pub id: NodeId,
    pub addr: String,
    pub raft: RaftConfig,
    pub store: Arc<RaftStore>,
    pub config: Arc<Config>,
}

pub async fn start_node(node_id: NodeId, bind_addr: SocketAddr) -> Result<()> {
    // Create a configuration for the raft instance.
    let config = Arc::new(Config::default().validate().unwrap());

    // Create a instance of where the Raft data will be stored.
    let store = Arc::new(RaftStore::default());

    // Create the network layer that will connect and communicate the raft instances and
    // will be used in conjunction with the store created above.
    let network = RaftNetworkConfig {};

    // Create a local raft instance.
    let raft = Raft::new(node_id, config.clone(), network, store.clone());

    // Create an application that will store all the instances created above, this will
    // be later used on the actix-web services.
    // let app = Data::new(ExampleApp {
    //     id: node_id,
    //     addr: http_addr.clone(),
    //     raft,
    //     store,
    //     config,
    // });

    Ok(())
}

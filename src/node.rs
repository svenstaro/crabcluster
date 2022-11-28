use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use openraft::BasicNode;
use openraft::Config;
use openraft::Raft;

use crate::raft_network::ExampleNetwork;
use crate::store::ExampleRequest;
use crate::store::ExampleResponse;
use crate::store::ExampleStore;

pub type ExampleNodeId = u64;

openraft::declare_raft_types!(
    /// Declare the type configuration for example K/V store.
    pub ExampleTypeConfig: D = ExampleRequest, R = ExampleResponse, NodeId = ExampleNodeId, Node = BasicNode
);

pub type ExampleRaft = Raft<ExampleTypeConfig, ExampleNetwork, Arc<ExampleStore>>;

// Representation of an application state. This struct can be shared around to share
// instances of raft, store and more.
pub struct ExampleApp {
    pub id: ExampleNodeId,
    pub addr: String,
    pub raft: ExampleRaft,
    pub store: Arc<ExampleStore>,
    pub config: Arc<Config>,
}

pub async fn start_node(node_id: u64, bind_addr: SocketAddr) -> Result<()> {
    // Create a configuration for the raft instance.
    let config = Arc::new(Config::default().validate().unwrap());

    // Create a instance of where the Raft data will be stored.
    let store = Arc::new(ExampleStore::default());

    // Create the network layer that will connect and communicate the raft instances and
    // will be used in conjunction with the store created above.
    let network = ExampleNetwork {};

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

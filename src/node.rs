use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use openraft::{BasicNode, Config, Raft};
use uuid::Uuid;

use crate::network::management::{add_learner, change_membership, get_id, init, metrics};
use crate::network::raft::{append, snapshot, vote};
use crate::network::user_api::{kv_read, kv_write};
use crate::raft_network::RaftNetworkClient;
use crate::store::{RaftRequest, RaftResponse, RaftStore};

pub type NodeId = Uuid;

openraft::declare_raft_types!(
    /// Declare the type configuration for K/V store.
    pub RaftTypeConfig: D = RaftRequest, R = RaftResponse, NodeId = NodeId, Node = BasicNode
);

pub type RaftConfig = Raft<RaftTypeConfig, RaftNetworkClient, Arc<RaftStore>>;

// Representation of an application state. This struct can be shared around to share
// instances of raft, store and more.
#[derive(Clone)]
pub struct RaftApp {
    pub id: NodeId,
    pub bind_addr: SocketAddr,
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
    let network = RaftNetworkClient {};

    // Create a local raft instance.
    let raft = Raft::new(node_id, config.clone(), network, store.clone());

    // Create an application that will store all the instances created above
    let app_state = RaftApp {
        id: node_id,
        bind_addr,
        raft,
        store,
        config,
    };

    let app = Router::new()
        .route("/init", get(init))
        .route("/raft-append", post(append))
        .route("/raft-snapshot", post(snapshot))
        .route("/raft-vote", post(vote))
        .route("/get-id", get(get_id))
        .route("/metrics", get(metrics))
        .route("/add-learner", post(add_learner))
        .route("/change-membership", post(change_membership))
        .route("/read", post(kv_read))
        .route("/write", post(kv_write))
        .with_state(app_state)
        .layer(tower_http::trace::TraceLayer::new_for_http());
    axum::Server::bind(&bind_addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

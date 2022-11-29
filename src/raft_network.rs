use async_trait::async_trait;
use openraft::error::{
    AppendEntriesError, InstallSnapshotError, NetworkError, RPCError, RemoteError, VoteError,
};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use openraft::{BasicNode, RaftNetwork, RaftNetworkFactory};
use serde::{de::DeserializeOwned, Serialize};

use crate::node::{NodeId, RaftTypeConfig};

pub struct RaftNetworkConfig {}

impl RaftNetworkConfig {
    pub async fn send_rpc<Req, Resp, Err>(
        &self,
        target: NodeId,
        target_node: &BasicNode,
        uri: &str,
        req: Req,
    ) -> Result<Resp, RPCError<NodeId, BasicNode, Err>>
    where
        Req: Serialize,
        Err: std::error::Error + DeserializeOwned,
        Resp: DeserializeOwned,
    {
        let addr = &target_node.addr;

        let url = format!("http://{}/{}", addr, uri);
        let client = reqwest::Client::new();

        let resp = client
            .post(url)
            .json(&req)
            .send()
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        let res: Result<Resp, Err> = resp
            .json()
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        res.map_err(|e| RPCError::RemoteError(RemoteError::new(target, e)))
    }
}

// NOTE: This could be implemented also on `Arc<ExampleNetwork>`, but since it's empty, implemented directly.
#[async_trait]
impl RaftNetworkFactory<RaftTypeConfig> for RaftNetworkConfig {
    type Network = RaftNetworkConnection;
    type ConnectionError = NetworkError;

    async fn new_client(
        &mut self,
        target: NodeId,
        node: &BasicNode,
    ) -> Result<Self::Network, Self::ConnectionError> {
        Ok(RaftNetworkConnection {
            owner: RaftNetworkConfig {},
            target,
            target_node: node.clone(),
        })
    }
}

pub struct RaftNetworkConnection {
    owner: RaftNetworkConfig,
    target: NodeId,
    target_node: BasicNode,
}

#[async_trait]
impl RaftNetwork<RaftTypeConfig> for RaftNetworkConnection {
    async fn send_append_entries(
        &mut self,
        req: AppendEntriesRequest<RaftTypeConfig>,
    ) -> Result<
        AppendEntriesResponse<NodeId>,
        RPCError<NodeId, BasicNode, AppendEntriesError<NodeId>>,
    > {
        self.owner
            .send_rpc(self.target, &self.target_node, "raft-append", req)
            .await
    }

    async fn send_install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<RaftTypeConfig>,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, BasicNode, InstallSnapshotError<NodeId>>,
    > {
        self.owner
            .send_rpc(self.target, &self.target_node, "raft-snapshot", req)
            .await
    }

    async fn send_vote(
        &mut self,
        req: VoteRequest<NodeId>,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, BasicNode, VoteError<NodeId>>> {
        self.owner
            .send_rpc(self.target, &self.target_node, "raft-vote", req)
            .await
    }
}

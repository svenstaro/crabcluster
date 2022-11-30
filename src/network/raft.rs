use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use openraft::raft::{AppendEntriesRequest, InstallSnapshotRequest, VoteRequest};

use crate::node::{NodeId, RaftApp, RaftTypeConfig};

pub async fn append(
    State(app_state): State<RaftApp>,
    Json(req): Json<AppendEntriesRequest<RaftTypeConfig>>,
) -> impl IntoResponse {
    let res = app_state.raft.append_entries(req).await;
    (StatusCode::CREATED, Json(res))
}

pub async fn snapshot(
    State(app_state): State<RaftApp>,
    Json(req): Json<InstallSnapshotRequest<RaftTypeConfig>>,
) -> impl IntoResponse {
    let res = app_state.raft.install_snapshot(req).await;
    (StatusCode::CREATED, Json(res))
}

pub async fn vote(
    State(app_state): State<RaftApp>,
    Json(req): Json<VoteRequest<NodeId>>,
) -> impl IntoResponse {
    let res = app_state.raft.vote(req).await;
    (StatusCode::CREATED, Json(res))
}

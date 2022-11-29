use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use openraft::raft::AppendEntriesRequest;

use crate::node::{RaftApp, RaftTypeConfig};

pub async fn append(
    State(app_state): State<RaftApp>,
    Json(req): Json<AppendEntriesRequest<RaftTypeConfig>>,
) -> impl IntoResponse {
    let res = app_state.raft.append_entries(req).await;
    (StatusCode::CREATED, Json(res))
}

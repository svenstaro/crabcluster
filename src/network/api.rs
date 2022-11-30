use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use openraft::error::Infallible;

use crate::{node::RaftApp, store::RaftRequest};

pub async fn write(
    State(app_state): State<RaftApp>,
    Json(req): Json<RaftRequest>,
) -> impl IntoResponse {
    let res = app_state.raft.client_write(req).await;
    (StatusCode::OK, Json(res))
}

pub async fn read(State(app_state): State<RaftApp>, Json(req): Json<String>) -> impl IntoResponse {
    let state_machine = app_state.store.state_machine.read().await;
    let key = req;
    let value = state_machine.data.get(&key).cloned();

    let res: Result<String, Infallible> = Ok(value.unwrap_or_default());
    (StatusCode::OK, Json(res))
}

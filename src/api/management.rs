use std::collections::BTreeMap;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use openraft::BasicNode;

use crate::node::RaftApp;

pub async fn init(State(app_state): State<RaftApp>) -> impl IntoResponse {
    // insert your application logic here
    let mut nodes = BTreeMap::new();
    nodes.insert(
        app_state.id,
        BasicNode {
            addr: app_state.bind_addr.to_string(),
        },
    );
    let res = app_state.raft.initialize(nodes).await;
    (StatusCode::CREATED, Json(res))
}

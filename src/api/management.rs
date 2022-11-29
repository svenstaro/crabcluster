//use axum::{
//    http::StatusCode,
//    response::IntoResponse,
//}
//
//pub async fn init(app: Data<ExampleApp>) ->  impl IntoResponse {
//    let mut nodes = BTreeMap::new();
//    nodes.insert(app.id, BasicNode { addr: app.addr.clone() });
//    let res = app.raft.initialize(nodes).await;
//
//    // this will be converted into a JSON response
//    // with a status code of `201 Created`
//    (StatusCode::CREATED, Json(res))
//}

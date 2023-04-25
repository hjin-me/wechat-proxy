use crate::backend::mp::MP;
use axum::body::{Body, Bytes};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;
use std::sync::Arc;

pub async fn message_send(Extension(mp): Extension<Arc<MP>>, b: Bytes) -> impl IntoResponse {
    let msg = String::from_utf8(b.to_vec()).unwrap();
    dbg!(mp.message_send(&msg).await.unwrap());
    Json(json!({"errcode" : 0, "errmsg" : "ok"}))
}

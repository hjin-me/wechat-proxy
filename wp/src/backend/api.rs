use crate::backend::mp::MP;
use axum::body::{Body, Bytes};
use axum::extract::{Path, Query};
use axum::http::header::HeaderMap;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use http::Request;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn message_send(Extension(mp): Extension<Arc<MP>>, b: Bytes) -> impl IntoResponse {
    let msg = String::from_utf8(b.to_vec()).unwrap();
    match mp.message_send(&msg).await {
        Ok(msg_id) => Json(json!({"errcode" : 0, "errmsg" : "ok", "msg_id" : msg_id})),
        Err(e) => Json(json!({"errcode" : -1, "errmsg" : e.to_string()})),
    }
}
#[derive(Deserialize, Serialize, Debug)]
pub struct RecallMsg {
    #[serde(rename = "msgid")]
    msg_id: String,
}

pub async fn message_recall(
    Extension(mp): Extension<Arc<MP>>,
    Json(q): Json<RecallMsg>,
) -> impl IntoResponse {
    match mp.message_recall(&q.msg_id).await {
        Ok(_) => Json(json!({"errcode" : 0, "errmsg" : "ok"})),
        Err(e) => Json(json!({"errcode" : -1, "errmsg" : e.to_string()})),
    }
}
pub async fn media_upload(
    Extension(mp): Extension<Arc<MP>>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    b: Bytes,
) -> impl IntoResponse {
    let mut qs = qstring::QString::new(vec![("debug", "1")]);
    for p in params.iter() {
        qs.add_pair((p.0, p.1));
    }

    let (c, s) = mp
        .proxy(
            &format!("https://123/cgi-bin/media/upload?{}", qs.to_string()),
            headers,
            b,
        )
        .await
        .unwrap();
    (
        c,
        [(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/json"),
        )],
        s,
    )
}

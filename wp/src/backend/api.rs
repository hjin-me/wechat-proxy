use crate::backend::chatglm::GLM;
use crate::backend::mp::callback::CallbackMessage::Text;
use crate::backend::mp::MP;

use axum::body::{Body, Bytes};
use axum::extract::{Path, Query};
use axum::http::header::HeaderMap;
use axum::response::IntoResponse;
use axum::{Extension, Json};

use crate::backend::context::ChatMgr;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{trace, warn};
use wechat_crypto::VerifyInfo;

pub async fn message_send(Extension(mp): Extension<Arc<MP>>, b: Bytes) -> impl IntoResponse {
    let msg = String::from_utf8(b.to_vec()).unwrap();
    match mp.proxy_message_send(&msg).await {
        Ok(msg_id) => Json(json!({"errcode" : 0, "errmsg" : "ok", "msgid" : msg_id})),
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
    let mut qs = qstring::QString::new::<&str, &str>(Vec::new());
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

#[derive(Deserialize, Serialize, Debug)]
pub struct ValidateQuery {
    msg_signature: String, //	是	企业微信加密签名，msg_signature结合了企业填写的token、请求中的timestamp、nonce参数、加密的消息体
    timestamp: i64,        //是	时间戳
    nonce: i64,            //是	随机数
    #[serde(rename = "echostr", default)]
    echo_str: String, //是	加密的字符串。需要解密得到消息内容明文，解密后有random、msg_len、msg、receiveid四个字段，其中msg即为消息内容明文
}

pub async fn validate_url(
    Extension(mp): Extension<Arc<MP>>,
    Query(q): Query<ValidateQuery>,
) -> impl IntoResponse {
    trace!("validate_url: {:?}", q);

    match mp.verify_url(
        &VerifyInfo {
            signature: q.msg_signature,
            timestamp: q.timestamp,
            nonce: q.nonce,
        },
        &q.echo_str,
    ) {
        Ok(echo) => echo,
        Err(e) => {
            warn!("url 验证失败: {:?}", e);
            "error".to_string()
        }
    }
}

pub async fn on_message(
    Extension(mp): Extension<Arc<MP>>,
    Extension(glm): Extension<Arc<GLM>>,
    Extension(chat_mgr): Extension<Arc<Mutex<ChatMgr>>>,
    Query(q): Query<ValidateQuery>,
    b: String,
) -> impl IntoResponse {
    trace!("on_message: q = {:?}", q);
    trace!("on_message: body = {:?}", b);
    match mp.handle_msg(
        &VerifyInfo {
            signature: q.msg_signature,
            timestamp: q.timestamp,
            nonce: q.nonce,
        },
        b.as_ref(),
    ) {
        Ok(xml) => {
            trace!("on_message: msg = {:?}", xml);
            if let Text(xml) = xml {
                if xml.content == "/clean" {
                    let mut m = chat_mgr.lock().await;
                    m.clear(&xml.from_user_name);
                    drop(m);
                    if let Err(e) = mp
                        .proxy_message_send(
                            &json!({
                                "touser": xml.from_user_name,
                                "msgtype": "text",
                                "agentid": 1,
                                "text": {
                                    "content": "让我们开始新的对话吧"
                                }
                            })
                            .to_string(),
                        )
                        .await
                    {
                        warn!(e = ?e, "proxy message send failed");
                    }
                } else {
                    glm.async_chat(
                        &xml.from_user_name,
                        &xml.content,
                        chat_mgr,
                        mp,
                        Some(Duration::from_secs(120)),
                    )
                    .await
                }
            }
        }
        Err(e) => {
            warn!("on_message 验证失败: {:?}", e);
        }
    }
}

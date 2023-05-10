use crate::backend::mp::MP;
use anyhow::Result;
use http::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Msg {
    from_user: String,
    query: String,
    // history: Vec<>
}
#[derive(Deserialize, Serialize, Debug)]
struct GLMResponse {
    response: String,
    history: Vec<Vec<String>>,
    status: u16,
}

#[derive(Clone)]
pub struct GLM {
    q: Arc<Mutex<VecDeque<Msg>>>,
    c: reqwest::Client,
    api: String,
}

impl GLM {
    pub fn new(api: &str) -> Self {
        Self {
            q: Arc::new(Mutex::new(VecDeque::new())),
            c: reqwest::Client::new(),
            api: api.to_string(),
        }
    }

    pub async fn chat(&self, from_user: &str, query: &str) {
        info!("glm chat: {:?}, {}", from_user, query);
        let mut q = self.q.lock().await;
        q.push_back(Msg {
            from_user: from_user.to_string(),
            query: query.to_string(),
        });
    }

    async fn _chat(&self, query: &str, history: Vec<Vec<String>>) -> Result<GLMResponse> {
        Ok(self
            .c
            .post(&self.api)
            .header(CONTENT_TYPE, "application/json")
            .json(&json!({"prompt": query, "history": history}))
            .send()
            .await?
            .json::<GLMResponse>()
            .await?)
    }

    pub fn queue_consumer(&mut self, mp: Arc<MP>) {
        let mut glm = self.clone();
        spawn(async move {
            loop {
                let mut q = glm.q.lock().await;
                if let Some(msg) = q.pop_front() {
                    info!("consumer msg: {:?}", msg);
                    drop(q);
                    let resp = glm._chat(&msg.query, vec![]).await;
                    match resp {
                        Ok(resp) => {
                            info!("glm response: {:?}", resp);
                            match mp
                                .proxy_message_send(
                                    &json!({
                                        "touser": msg.from_user,
                                        "msgtype": "text",
                                        "agentid": 1,
                                        "text": {
                                            "content": resp.response
                                        }
                                    })
                                    .to_string(),
                                )
                                .await
                            {
                                Ok(_) => {
                                    info!("glm response sent");
                                }
                                Err(e) => {
                                    warn!("glm response send error: {:?}", e);
                                }
                            };
                        }
                        Err(e) => {
                            warn!("glm error: {:?}", e);
                        }
                    }
                } else {
                    drop(q);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test]
    async fn test_chat() -> Result<()> {
        let glm = GLM::new("");
        glm._chat("你好", vec![]).await?;
        Ok(())
    }
}

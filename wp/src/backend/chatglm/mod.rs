use crate::backend::context::ChatMgr;
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
    prompt_prefix: String,
}

impl GLM {
    pub fn new(api: &str, prompt_prefix: &str) -> Self {
        Self {
            q: Arc::new(Mutex::new(VecDeque::new())),
            c: reqwest::Client::new(),
            api: api.to_string(),
            prompt_prefix: prompt_prefix.to_string(),
        }
    }

    pub async fn chat(&self, from_user: &str, query: &str) -> usize {
        info!(q = query, u = from_user, "glm chat");
        let mut q = self.q.lock().await;
        let l = q.len();
        q.push_back(Msg {
            from_user: from_user.to_string(),
            query: query.to_string(),
        });
        l
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

    pub fn queue_consumer(&mut self, mp: Arc<MP>, mut chat_mgr: ChatMgr) {
        let glm = self.clone();
        spawn(async move {
            loop {
                let mut q = glm.q.lock().await;
                if let Some(msg) = q.pop_front() {
                    info!("consumer msg: {:?}", msg);
                    drop(q);
                    let query = format!("{}\n{}", glm.prompt_prefix, msg.query);
                    let history = chat_mgr
                        .get(&msg.from_user)
                        .map(|c| c.history())
                        .unwrap_or(vec![]);
                    let resp = glm._chat(&query, history).await;
                    match resp {
                        Ok(resp) => {
                            info!("history: {:?}", resp.history);
                            info!(
                                q = query,
                                a = resp.response,
                                u = msg.from_user,
                                t = "glm",
                                h = ?resp.history,
                                "glm response"
                            );
                            chat_mgr.add(&msg.from_user, &msg.query, &resp.response, 0);
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
        let glm = GLM::new("", "");
        glm._chat("你好", vec![vec!["他好".to_string(), "我也好".to_string()]])
            .await?;
        Ok(())
    }
}

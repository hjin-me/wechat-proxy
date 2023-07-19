use crate::backend::context::ChatMgr;
use crate::backend::mp::MP;
use anyhow::{anyhow, Result};
use openai_api_rust::chat::ChatApi;
use openai_api_rust::completions::Completion;
use openai_api_rust::OpenAI;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, trace, warn};

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
    m: Arc<Mutex<u32>>,
    c: reqwest::Client,
    // api: String,
    openai: OpenAI,
}

impl GLM {
    pub fn new(api: &str) -> Self {
        let auth = openai_api_rust::Auth::new("none");
        let openai = OpenAI::new(auth, api);
        Self {
            m: Arc::new(Mutex::new(0)),
            c: reqwest::Client::new(),
            openai,
        }
    }
    pub async fn async_chat(
        &self,
        from_user: &str,
        query: &str,
        chat_mgr: Arc<Mutex<ChatMgr>>,
        mp: Arc<MP>,
        timeout: Option<Duration>,
    ) {
        let glm = self.clone();
        let from_user = from_user.to_string();
        let query = query.to_string();
        tokio::spawn(async move {
            glm.chat(&from_user, &query, chat_mgr, mp, timeout)
                .await
                .unwrap_or_else(|e| {
                    warn!(q = query, u = from_user, "glm chat error: {:?}", e);
                })
        });
    }

    pub async fn chat(
        &self,
        from_user: &str,
        query: &str,
        chat_mgr: Arc<Mutex<ChatMgr>>,
        mp: Arc<MP>,
        timeout: Option<Duration>,
    ) -> Result<()> {
        info!(q = query, u = from_user, "glm chat");
        // process
        // 2 等待获取锁
        // 1 拿到锁了，正在请求
        // 0 结束了
        let processing = Arc::new(Mutex::new(2));
        {
            let processing = processing.clone();
            let from_user = from_user.to_string();
            let mp = mp.clone();
            tokio::spawn(async move {
                let d = Duration::from_secs(13);
                tokio::time::sleep(d).await;
                loop {
                    let p = processing.lock().await;
                    let resp_msg = match *p {
                        1 => "小秘书😣正在燃烧为数不多[求赞助]的脑细胞帮你回答问题，莫急莫急",
                        2 => "小秘书😣忙得焦头烂额，多等一会儿，忙完马上回复你",
                        _ => return,
                    };
                    let _ = mp
                        .proxy_message_send(
                            &json!({
                                "touser": from_user,
                                "msgtype": "text",
                                "agentid": 1,
                                "text": {
                                    "content": resp_msg
                                }
                            })
                            .to_string(),
                        )
                        .await;

                    tokio::time::sleep(d * 3).await;
                }
            });
        }
        let _m = self.m.lock().await;
        {
            let mut p = processing.lock().await;
            *p = 1;
        }
        // *m += 1u32;

        let begin = time::OffsetDateTime::now_utc();
        let m_handler = {
            let from_user = from_user.to_string();
            let chat_mgr = chat_mgr.clone();
            let glm = self.clone();
            let query = query.to_string();
            tokio::spawn(tokio::time::timeout(
                timeout.unwrap_or(Duration::from_secs(60)),
                async move {
                    let m = chat_mgr.lock().await;
                    let history = m.get(&from_user).map(|c| c.history()).unwrap_or(vec![]);
                    drop(m);
                    glm._chat(&query, history).await
                },
            ))
        };

        let m_ret = m_handler.await;
        {
            let mut p = processing.lock().await;
            *p = 0;
        }
        let cost_during = time::OffsetDateTime::now_utc() - begin;
        let m_ret = match m_ret {
            Err(e) => {
                warn!(q = query, u = from_user, "glm thread error: {:?}", e);
                Err(anyhow!("{:?}", e))
            }
            Ok(r) => match r {
                Err(e) => {
                    warn!(q = query, u = from_user, "glm timeout: {:?}", e);
                    Err(anyhow!("{:?}", e))
                }
                Ok(r) => r,
            },
        };

        let resp_msg = match m_ret {
            Ok(resp) => {
                info!(
                    q = query,
                    u = from_user,
                    t = "glm",
                    c = cost_during.whole_seconds(),
                    "glm response"
                );
                // let mut chat_mgr = chat_mgr.lock().await;
                // chat_mgr.add(
                //     from_user,
                //     query,
                //     &resp.response,
                //     time::OffsetDateTime::now_utc().unix_timestamp(),
                // );
                let r = resp
                    .choices
                    .get(0)
                    .ok_or(anyhow!("glm response error"))?
                    .message
                    .clone()
                    .ok_or(anyhow!("glm response error"))?;
                format!(
                    "{}\n\n> 对话耗时：{}s\n> /clean 重新开始聊天",
                    r.content,
                    cost_during.whole_seconds()
                )
            }
            Err(e) => {
                warn!(
                    q = query,
                    u = from_user,
                    c = cost_during.whole_seconds(),
                    "glm error: {:?}",
                    e
                );
                "ChatGLM 回答失败，请稍后再试试".to_string()
            }
        };
        match mp
            .proxy_message_send(
                &json!({
                    "touser": from_user,
                    "msgtype": "markdown",
                    "agentid": 1,
                    "markdown": {
                        "content": resp_msg
                    }
                })
                .to_string(),
            )
            .await
        {
            Ok(_) => {
                trace!("glm response sent");
            }
            Err(e) => {
                warn!("glm response send error: {:?}", e);
            }
        };
        Ok(())
    }

    async fn _chat(&self, query: &str, history: Vec<Vec<String>>) -> Result<Completion> {
        let body = openai_api_rust::chat::ChatBody {
            model: "chatglm2-6b".to_string(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            n: Some(2),
            stream: Some(false),
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            messages: vec![openai_api_rust::Message {
                role: openai_api_rust::Role::User,
                content: query.to_string(),
            }],
        };
        self.openai
            .chat_completion_create(&body)
            .map_err(|e| anyhow::Error::msg("openai chat completion create error").context(e))
        // let message = &choice[0].message.as_ref().unwrap();
        // dbg!(message);
        // Ok(self
        //     .c
        //     .post(&self.api)
        //     .header(CONTENT_TYPE, "application/json")
        //     .json(&json!({"prompt": query, "history": history}))
        //     .send()
        //     .await?
        //     .json::<GLMResponse>()
        //     .await?)
    }

    // pub fn queue_consumer(&mut self, mp: Arc<MP>, mut chat_mgr: ChatMgr) {
    //     let glm = self.clone();
    // spawn(async move {
    //     loop {
    //         let mut q = glm.q.lock().await;
    //         if let Some(msg) = q.pop_front() {
    //             trace!("consumer msg: {:?}", msg);
    //             drop(q);
    //             if msg.query == "/clean" {
    //                 chat_mgr.clear(&msg.from_user);
    //                 if let Err(e) = mp
    //                     .proxy_message_send(
    //                         &json!({
    //                             "touser": msg.from_user,
    //                             "msgtype": "text",
    //                             "agentid": 1,
    //                             "text": {
    //                                 "content": "让我们开始新的对话吧"
    //                             }
    //                         })
    //                         .to_string(),
    //                     )
    //                     .await
    //                 {
    //                     warn!(e = ?e, "proxy message send failed");
    //                 }
    //                 continue;
    //             }
    //
    //             let query = if glm.prompt_prefix.trim().is_empty() {
    //                 msg.query.clone()
    //             } else {
    //                 format!("{}\n{}", glm.prompt_prefix, msg.query)
    //             };
    //             let history = chat_mgr
    //                 .get(&msg.from_user)
    //                 .map(|c| c.history())
    //                 .unwrap_or(vec![]);
    //             let begin = time::OffsetDateTime::now_utc();
    //             let resp = match tokio::time::timeout(
    //                 std::time::Duration::from_secs(60 * 2),
    //                 glm._chat(&query, history),
    //             )
    //             .await
    //             {
    //                 Ok(r) => r,
    //                 Err(e) => {
    //                     warn!("glm timeout: {:?}", e);
    //                     Err(anyhow::anyhow!("请求 ChatGLM 超过2分钟"))
    //                 }
    //             };
    //             let cost_during = time::OffsetDateTime::now_utc() - begin;
    //             let resp_msg = match resp {
    //                 Ok(resp) => {
    //                     info!(
    //                         q = query,
    //                         a = resp.response,
    //                         u = msg.from_user,
    //                         t = "glm",
    //                         h = ?resp.history,
    //                         c = cost_during.whole_seconds(),
    //                         "glm response"
    //                     );
    //                     chat_mgr.add(
    //                         &msg.from_user,
    //                         &msg.query,
    //                         &resp.response,
    //                         time::OffsetDateTime::now_utc().unix_timestamp(),
    //                     );
    //                     format!(
    //                         "{}\n\n> 对话耗时：{}s\n> /clean 重新开始聊天",
    //                         resp.response,
    //                         cost_during.whole_seconds()
    //                     )
    //                 }
    //                 Err(e) => {
    //                     warn!(
    //                         q = query,
    //                         u = msg.from_user,
    //                         c = cost_during.whole_seconds(),
    //                         "glm error: {:?}",
    //                         e
    //                     );
    //                     "ChatGLM 回答失败，请稍后再试试".to_string()
    //                 }
    //             };
    //             match mp
    //                 .proxy_message_send(
    //                     &json!({
    //                         "touser": msg.from_user,
    //                         "msgtype": "markdown",
    //                         "agentid": 1,
    //                         "markdown": {
    //                             "content": resp_msg
    //                         }
    //                     })
    //                     .to_string(),
    //                 )
    //                 .await
    //             {
    //                 Ok(_) => {
    //                     trace!("glm response sent");
    //                 }
    //                 Err(e) => {
    //                     warn!("glm response send error: {:?}", e);
    //                 }
    //             };
    //         } else {
    //             drop(q);
    //             tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    //         }
    //     }
    // });
    // }
}

#[cfg(test)]
mod test {
    use super::*;
    use tracing::error;
    #[tokio::test]
    async fn test_chat() -> Result<()> {
        let glm = GLM::new("");
        glm._chat("你好", vec![vec!["他好".to_string(), "我也好".to_string()]])
            .await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_parallel() -> Result<()> {
        let h1 = tokio::spawn(tokio::time::timeout(
            std::time::Duration::from_secs(5),
            async {
                info!("begin 4 secs");
                tokio::time::sleep(Duration::from_secs(4)).await;
                info!("after 4 secs");
            },
        ));
        let h2 = tokio::spawn(async {
            warn!("begin 1 secs");
            tokio::time::sleep(Duration::from_secs(1)).await;
            warn!("after 1 secs");
            tokio::time::sleep(Duration::from_secs(1)).await;
            warn!("after 1 secs");
        });
        error!("start");
        h2.await?;
        h1.await?;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn my_test() {
        // 创建两个异步函数
        let task1 = async {
            info!(
                "Task 1 started on thread {:?}.",
                std::thread::current().id()
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            info!(
                "Task 1 finished on thread {:?}.",
                std::thread::current().id()
            );
            42
        };
        let task2 = async {
            info!(
                "Task 2 started on thread {:?}.",
                std::thread::current().id()
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            info!(
                "Task 2 finished on thread {:?}.",
                std::thread::current().id()
            );
            "Hello"
        };

        // 在 Tokio 运行时中并行地执行两个异步函数，并获取它们的 JoinHandle
        let handle1 = tokio::spawn(task1);
        let handle2 = tokio::spawn(task2);

        // 等待两个异步函数的完成，并获取它们的返回值
        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        // 打印结果
        println!("Result 1: {}", result1);
        println!("Result 2: {}", result2);
    }
}

pub mod callback;
mod client;
pub mod crypt;
mod media;
mod msg;

use crate::backend::mp::callback::CallbackMessage;
use crate::backend::mp::crypt::{verify_url, VerifyInfo};
use anyhow::{anyhow, Result};
use axum::body::Bytes;
use http::{HeaderMap, StatusCode};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, info};

struct Token {
    content: String,
    expires_after: time::OffsetDateTime,
}
pub struct MP {
    corp_id: String,
    corp_secret: String,
    agent_id: i64,
    access_token: RwLock<Token>,
    client: reqwest::Client,
    aek_key: Vec<u8>,
    token: String,
}

impl MP {
    pub fn new(
        corp_id: &str,
        corp_secret: &str,
        agent_id: &i64,
        encoded_aes_key: &str,
        token: &str,
    ) -> Self {
        let aek_key = crypt::decode_aes_key(encoded_aes_key).expect("解码企业微信 AES key 失败");
        Self {
            corp_id: corp_id.to_string(),
            corp_secret: corp_secret.to_string(),
            agent_id: agent_id.clone(),
            access_token: RwLock::new(Token {
                content: "".to_string(),
                expires_after: time::OffsetDateTime::now_utc(),
            }),
            client: reqwest::Client::new(),
            aek_key,
            token: token.to_string(),
        }
    }
    async fn refresh_token(&self) -> Result<()> {
        info!("refresh_token");
        let (access_token, expires_in) =
            client::get_access_token(&self.corp_id, &self.corp_secret).await?;
        let mut w = self.access_token.write().await;
        (*w).content = access_token;
        (*w).expires_after =
            time::OffsetDateTime::now_utc() + time::Duration::seconds(expires_in - 30);
        Ok(())
    }

    pub async fn get_token(&self) -> Result<String> {
        let token = self.access_token.read().await;
        if token.expires_after < time::OffsetDateTime::now_utc() {
            drop(token);
            self.refresh_token().await?;
        }
        let r = self.access_token.read().await;
        Ok(r.content.clone())
    }

    pub async fn message_send(&self, msg: &str) -> Result<String> {
        let token = self.get_token().await?;
        let msg_id = msg::send_msg(&self.client, &token, &self.agent_id, msg).await?;
        Ok(msg_id)
    }
    pub async fn message_recall(&self, msg_id: &str) -> Result<()> {
        let token = self.get_token().await?;
        msg::recall_msg(&self.client, &token, msg_id).await?;
        Ok(())
    }
    pub async fn proxy(
        &self,
        uri: &str,
        headers: HeaderMap,
        b: Bytes,
    ) -> Result<(StatusCode, String)> {
        let token = self.get_token().await?;
        let mut h = HeaderMap::new();
        for (k, v) in headers.iter() {
            debug!("{}: {}", k, v.to_str()?);
            match k {
                &http::header::HOST => {}
                &http::header::ACCEPT_ENCODING => {}
                _ => {
                    h.insert(k, v.clone());
                }
            }
        }

        // u.query_pairs()
        let u = rebuild_url(uri, &token).await?;
        info!("proxy url: {}", u);
        info!("proxy headers: {:?}", h);
        let r = self.client.post(u).headers(h).body(b).send().await?;
        let code = r.status();
        let body = r.text().await?;
        info!("proxy response: {} {}", code, body);

        Ok((code, body))
    }
}

// 服务器回复消息
impl MP {
    pub fn verify_url(&self, q: &VerifyInfo, echo_str: &str) -> Result<String> {
        Ok(verify_url(
            self.token.as_str(),
            q,
            &echo_str,
            &self.aek_key,
            &self.corp_id,
        )?)
    }
    pub fn handle_msg(&self, q: &VerifyInfo, b: &str) -> Result<CallbackMessage> {
        let msg = callback::decrypt_message(&self.aek_key, &self.corp_id, &self.token, q, b)?;
        dbg!(&msg);
        Ok(msg)
    }
}

async fn rebuild_url(uri: &str, token: &str) -> Result<String> {
    let mut u = url::Url::parse(uri)?;
    u.set_host(Some("qyapi.weixin.qq.com"))?;
    u.set_scheme("https").map_err(|_| anyhow!(""))?;
    u.set_port(None).map_err(|_| anyhow!(""))?;
    let q = u.query().unwrap_or("");
    let qs = qstring::QString::from(q);
    let mut nq = qstring::QString::new(vec![("access_token", token)]);
    for (k, v) in qs.into_pairs().iter() {
        debug!("{}: {}", k, v);
        if k == "access_token" {
            continue;
        }
        nq.add_pair((k, v))
    }
    u.set_query(Some(&nq.to_string()));
    Ok(u.to_string())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::Config;
    use std::fs;

    #[tokio::test]
    async fn test_url() -> Result<()> {
        let r = dbg!(
            rebuild_url(
                "http://127.0.0.1:3000/cgi-bin/media/upload?access_token=ACCESS_TOKEN&type=image",
                "666"
            )
            .await?
        );
        assert_eq!(
            r,
            "https://qyapi.weixin.qq.com/cgi-bin/media/upload?access_token=666&type=image"
        );

        // dbg!(rebuild_url("/cgi-bin/media/upload?", "666").await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_token() -> Result<()> {
        let contents = fs::read_to_string("./config.toml").expect("读取配置失败");
        let serv_conf: Config = toml::from_str(contents.as_str()).unwrap();
        let mp = MP::new(
            &serv_conf.corp_id,
            &serv_conf.corp_secret,
            &serv_conf.agent_id,
            &serv_conf.encoded_aes_key,
            &serv_conf.token,
        );
        let t1 = dbg!(mp.get_token().await?);
        let t2 = dbg!(mp.get_token().await?);
        let t3 = dbg!(mp.get_token().await?);
        assert_eq!(t1, t2);
        assert_eq!(t2, t3);
        Ok(())
    }

    #[tokio::test]
    async fn test_message_send() -> Result<()> {
        let msg = r#"{
  "touser": "SongSong",
  "msgtype": "text",
  "agentid": 10,
  "text": {
    "content": "content"
  }
}
        "#;
        let contents = fs::read_to_string("./config.toml").expect("读取配置失败");
        let serv_conf: Config = toml::from_str(contents.as_str()).unwrap();
        let mp = MP::new(
            &serv_conf.corp_id,
            &serv_conf.corp_secret,
            &serv_conf.agent_id,
            &serv_conf.encoded_aes_key,
            &serv_conf.token,
        );
        let msg_id = dbg!(mp.message_send(msg).await?);

        dbg!(mp.message_recall(&msg_id).await?);
        Ok(())
    }
}

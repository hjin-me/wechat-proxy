mod client;
mod media;
mod msg;

use anyhow::Result;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{debug, info};

struct Token {
    content: String,
    expires_after: time::OffsetDateTime,
}
struct MP {
    corp_id: String,
    corp_secret: String,
    agent_id: i64,
    access_token: RwLock<Token>,
    client: reqwest::Client,
}

impl MP {
    fn new(corp_id: &str, corp_secret: &str, agent_id: &i64) -> Self {
        Self {
            corp_id: corp_id.to_string(),
            corp_secret: corp_secret.to_string(),
            agent_id: agent_id.clone(),
            access_token: RwLock::new(Token {
                content: "".to_string(),
                expires_after: time::OffsetDateTime::now_utc(),
            }),
            client: reqwest::Client::new(),
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

    pub async fn message_send(&self, msg: &str) -> Result<()> {
        let token = self.get_token().await?;
        msg::send_msg(&self.client, &token, &self.agent_id, msg).await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::Config;
    use std::fs;

    #[tokio::test]
    async fn test_get_token() -> Result<()> {
        let contents = fs::read_to_string("./config.toml").expect("读取配置失败");
        let serv_conf: Config = toml::from_str(contents.as_str()).unwrap();
        let mp = MP::new(
            &serv_conf.corp_id,
            &serv_conf.corp_secret,
            &serv_conf.agent_id,
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
        );
        mp.message_send(msg).await?;
        Ok(())
    }
}

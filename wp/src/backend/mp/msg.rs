use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone)]
enum MsgType {
    Text,
    Image,
    // Voice,
    // Video,
    // File,
    // Textcard,
    // News,
    // Mpnews,
    // Markdown,
    // MiniprogramNotice,
    // Taskcard,
    // InteractiveTaskcard,
    // TemplateCard,
}
impl Display for MsgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MsgType::Text => write!(f, "text"),
            MsgType::Image => write!(f, "image"),
        }
    }
}

impl From<String> for MsgType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "text" => MsgType::Text,
            "image" => MsgType::Image,
            _ => MsgType::Text,
        }
    }
}
impl MsgType {
    fn to_string(&self) -> String {
        match self {
            MsgType::Text => "text".to_string(),
            MsgType::Image => "image".to_string(),
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            MsgType::Text => "text",
            MsgType::Image => "image",
        }
    }
}
impl<'de> Deserialize<'de> for MsgType {
    fn deserialize<D>(deserializer: D) -> Result<MsgType, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(MsgType::from(s))
    }
}
impl Serialize for MsgType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        // S::Error: std::error::Error,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TextContent {
    content: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct ImageContent {
    media_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum SendMsgReq {
    TextMsgReq(SendTextMsgReq),
    ImageMsgReq(SendImageMsgReq),
}

#[derive(Serialize, Deserialize, Debug)]
struct SendImageMsgReq {
    #[serde(rename = "touser", skip_serializing_if = "Option::is_none")]
    pub to_user: Option<String>,
    #[serde(rename = "toparty", skip_serializing_if = "Option::is_none")]
    pub to_party: Option<String>,
    #[serde(rename = "totag", skip_serializing_if = "Option::is_none")]
    pub to_tag: Option<String>,
    #[serde(rename = "msgtype")]
    pub msg_type: MsgType,
    #[serde(rename = "agentid")]
    pub agent_id: i64,
    image: ImageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    safe: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_id_trans: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_duplicate_check: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duplicate_check_interval: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SendTextMsgReq {
    #[serde(rename = "touser", skip_serializing_if = "Option::is_none")]
    pub to_user: Option<String>,
    #[serde(rename = "toparty", skip_serializing_if = "Option::is_none")]
    pub to_party: Option<String>,
    #[serde(rename = "totag", skip_serializing_if = "Option::is_none")]
    pub to_tag: Option<String>,
    #[serde(rename = "msgtype")]
    pub msg_type: MsgType,
    #[serde(rename = "agentid")]
    pub agent_id: i64,
    pub text: TextContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    safe: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_id_trans: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_duplicate_check: Option<i8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duplicate_check_interval: Option<i32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct SendMsgResponse {
    #[serde(rename = "errcode")]
    err_code: i32, //	返回码
    #[serde(rename = "errmsg")]
    err_msg: String, //	对返回码的文本描述内容
                     // invaliduser	不合法的userid，不区分大小写，统一转为小写
                     // invalidparty	不合法的partyid
                     // invalidtag	不合法的标签id
                     // unlicenseduser	没有基础接口许可(包含已过期)的userid
                     // msgid	消息id，用于撤回应用消息
                     // response_code	仅消息类型为“按钮交互型”，“投票选择型”和“多项选择型”的模板卡片消息返回，应用可使用response_code调用更新模版卡片消息接口，72小时内有效，且只能使用一次
}
pub async fn send_msg(
    client: &reqwest::Client,
    token: &str,
    agent_id: &i64,
    msg: &str,
) -> Result<()> {
    let body = match serde_json::from_str::<SendMsgReq>(msg)? {
        SendMsgReq::TextMsgReq(mut q) => {
            q.agent_id = agent_id.clone();
            SendMsgReq::TextMsgReq(q)
        }
        SendMsgReq::ImageMsgReq(mut q) => {
            q.agent_id = agent_id.clone();
            SendMsgReq::ImageMsgReq(q)
        }
    };

    let api = format!(
        "https://qyapi.weixin.qq.com/cgi-bin/message/send?access_token={}",
        token
    );

    let res = client
        .post(api)
        .body(serde_json::to_string(&body)?)
        .send()
        .await?
        .json::<SendMsgResponse>()
        .await?;
    if res.err_code != 0 {
        return Err(anyhow!(
            "发送消息失败 error: [{}] {}",
            res.err_code,
            res.err_msg
        ));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::mp::client::get_access_token;
    use crate::backend::Config;
    use std::fs;

    #[tokio::test]
    async fn test_gat() -> Result<()> {
        dbg!(std::env::current_dir()?);
        let contents = fs::read_to_string("./config.toml").expect("读取配置失败");
        let serv_conf: Config = toml::from_str(contents.as_str()).unwrap();

        let (token, _) = dbg!(get_access_token(&serv_conf.corp_id, &serv_conf.corp_secret).await?);
        send_msg(&reqwest::Client::new(),&token, &serv_conf.agent_id, r#"{ "touser" : "SongSong", "msgtype" : "text", "agentid" : 1, "text" : { "content" : "content" } }"#).await?;
        Ok(())
    }

    #[test]
    fn test_json() {
        dbg!(serde_json::from_str::<MsgType>("\"image\"").unwrap());
        dbg!(serde_json::to_string(&MsgType::Image).unwrap());

        dbg!(serde_json::from_str::<SendMsgReq>(
            r#"{
  "touser": "abc",
  "msgtype": "text",
  "agentid": 14,
  "text": {
    "content": "content"
  }
}
        "#
        )
        .unwrap());
        let x = dbg!(serde_json::from_str::<SendMsgReq>(
            r#"{
  "touser": "abc",
    "msgtype" : "image",
   "agentid" : 1,
   "image" : {
        "media_id" : "MEDIA_ID"
   }
}
        "#
        )
        .unwrap());

        assert_eq!(
            serde_json::to_string(&x).unwrap(),
            r#"{"touser":"abc","msgtype":"image","agentid":1,"image":{"media_id":"MEDIA_ID"}}"#
        );
    }
}

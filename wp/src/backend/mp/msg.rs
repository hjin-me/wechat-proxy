use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone)]
enum MsgType {
    Text,
    Image,
    Voice,
    Video,
    File,
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
            MsgType::Voice => write!(f, "voice"),
            MsgType::Video => write!(f, "video"),
            MsgType::File => write!(f, "file"),
        }
    }
}

impl From<String> for MsgType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "text" => MsgType::Text,
            "image" => MsgType::Image,
            "voice" => MsgType::Voice,
            "video" => MsgType::Video,
            "file" => MsgType::File,
            _ => MsgType::Text,
        }
    }
}
impl MsgType {
    fn to_string(&self) -> String {
        match self {
            MsgType::Text => "text".to_string(),
            MsgType::Image => "image".to_string(),
            MsgType::Voice => "voice".to_string(),
            MsgType::Video => "video".to_string(),
            MsgType::File => "file".to_string(),
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            MsgType::Text => "text",
            MsgType::Image => "image",
            MsgType::Voice => "voice",
            MsgType::Video => "video",
            MsgType::File => "file",
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
struct MediaContent {
    media_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum SendMsgReq {
    TextMsgReq(SendTextMsgReq),
    ImageMsgReq(SendImageMsgReq),
    VoiceMsgReq(SendVoiceMsgReq),
    VideoMsgReq(SendVideoMsgReq),
    FileMsgReq(SendFileMsgReq),
}

#[derive(Serialize, Deserialize, Debug)]
struct SendMsgCommon {
    #[serde(rename = "touser", skip_serializing_if = "Option::is_none")]
    pub to_user: Option<String>,
    #[serde(rename = "toparty", skip_serializing_if = "Option::is_none")]
    pub to_party: Option<String>,
    #[serde(rename = "totag", skip_serializing_if = "Option::is_none")]
    pub to_tag: Option<String>,
    #[serde(rename = "msgtype")]
    pub msg_type: MsgType,
    #[serde(rename = "agentid", default)]
    pub agent_id: i64,
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
struct SendImageMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    image: MediaContent,
}
#[derive(Serialize, Deserialize, Debug)]
struct SendTextMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    text: TextContent,
}
#[derive(Serialize, Deserialize, Debug)]
struct SendVoiceMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    voice: MediaContent,
}
#[derive(Serialize, Deserialize, Debug)]
struct SendVideoMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    video: MediaContent,
}
#[derive(Serialize, Deserialize, Debug)]
struct SendFileMsgReq {
    #[serde(flatten)]
    common: SendMsgCommon,
    file: MediaContent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SendMsgResponse {
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
            q.common.msg_type = MsgType::Text;
            q.common.agent_id = agent_id.clone();
            SendMsgReq::TextMsgReq(q)
        }
        SendMsgReq::ImageMsgReq(mut q) => {
            q.common.msg_type = MsgType::Image;
            q.common.agent_id = agent_id.clone();
            SendMsgReq::ImageMsgReq(q)
        }
        SendMsgReq::VoiceMsgReq(mut q) => {
            q.common.msg_type = MsgType::Voice;
            q.common.agent_id = agent_id.clone();
            SendMsgReq::VoiceMsgReq(q)
        }
        SendMsgReq::VideoMsgReq(mut q) => {
            q.common.msg_type = MsgType::Video;
            q.common.agent_id = agent_id.clone();
            SendMsgReq::VideoMsgReq(q)
        }
        SendMsgReq::FileMsgReq(mut q) => {
            q.common.msg_type = MsgType::File;
            q.common.agent_id = agent_id.clone();
            SendMsgReq::FileMsgReq(q)
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
    use assert_json_diff::assert_json_eq;
    use serde_json::json;
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
        let cases = vec![
            (
                r#"{ "touser": "abc", "msgtype": "text", "text": { "content": "content" }}"#,
                r#"{"touser":"abc","msgtype":"text","agentid":0,"text":{"content":"content"}}"#,
            ),
            (
                r#"{ "touser": "abc", "msgtype" : "image", "image" : { "media_id" : "MEDIA_ID" }}"#,
                r#"{"touser":"abc","msgtype":"image","agentid":0,"image":{"media_id":"MEDIA_ID"}}"#,
            ),
            (
                r#"{
  "touser": "UserID1|UserID3",
  "toparty": "PartyID1|PartyID2",
  "totag": "TagID1 | TagID2",
  "msgtype": "voice",
  "agentid": 3,
  "voice": {
    "media_id": "MEDIA_ID"
  },
  "enable_duplicate_check": 0,
  "duplicate_check_interval": 1800
}"#,
                r#"{"touser":"UserID1|UserID3","toparty":"PartyID1|PartyID2","totag":"TagID1 | TagID2","msgtype":"voice","agentid":3,"enable_duplicate_check":0,"duplicate_check_interval":1800,"voice":{"media_id":"MEDIA_ID"}}"#,
            ),
            (
                r#"{
   "touser" : "UserIID3",
   "toparty" : "ParrtyID2",
   "totag" : "TaID2",
   "msgtype" : "video",
   "agentid" : 1,
   "video" : {
        "media_id" : "MEDIA_ID",
        "title" : "Title",
       "description" : "Description"
   },
   "safe":0,
   "enable_duplicate_check": 0,
   "duplicate_check_interval": 1800
}"#,
                r#"{
  "touser": "UserIID3",
  "toparty": "ParrtyID2",
  "totag": "TaID2",
  "msgtype": "video",
  "agentid": 1,
  "safe": 0,
  "enable_duplicate_check": 0,
  "duplicate_check_interval": 1800,
  "video": {
    "media_id": "MEDIA_ID",
    "title": "Title",
    "description": "Description"
  }
}"#,
            ),
            (
                r#"{
   "touser" : "UserID1",
   "toparty" : "PartyID1|",
   "totag" : "TagID1 | TagID2",
   "msgtype" : "file",
   "agentid" : 1,
   "file" : {
        "media_id" : "1Yv-zXfHjSjU-7LH-GwtYqDGS-zz6w22KmWAT5COgP7o"
   },
   "safe":0,
   "enable_duplicate_check": 0,
   "duplicate_check_interval": 1800
}"#,
                r#"{"touser":"UserID1","toparty":"PartyID1|","totag":"TagID1 | TagID2","msgtype":"file","agentid":1,"safe":0,"enable_duplicate_check":0,"duplicate_check_interval":1800,"file":{"media_id":"1Yv-zXfHjSjU-7LH-GwtYqDGS-zz6w22KmWAT5COgP7o"}}"#,
            ),
        ];
        for x in cases {
            let t = serde_json::from_str::<SendMsgReq>(x.0).unwrap();
            let s = serde_json::to_string(&t).unwrap();
            let vl = serde_json::from_str::<serde_json::Value>(&s).unwrap();
            let vr = serde_json::from_str::<serde_json::Value>(x.1).unwrap();
            assert_json_eq!(vl, vr);
        }
    }
}

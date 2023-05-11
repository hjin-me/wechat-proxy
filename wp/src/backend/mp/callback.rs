use anyhow::{anyhow, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use wechat_crypto::{calc_signature, decrypt, parse_plain_text, VerifyInfo};

pub fn check_sign(token: &str, q: &VerifyInfo, data: &str) -> bool {
    let s = calc_signature(
        token,
        q.timestamp.to_string().as_str(),
        q.nonce.to_string().as_str(),
        data,
    );
    s == q.signature
}
// <xml>
//    <ToUserName><![CDATA[toUser]]></ToUserName>
//    <AgentID><![CDATA[toAgentID]]></AgentID>
//    <Encrypt><![CDATA[msg_encrypt]]></Encrypt>
// </xml>
#[derive(Deserialize, Debug)]
struct EncryptedXML {
    #[serde(rename = "ToUserName")]
    to_user_name: String,
    #[serde(rename = "AgentID")]
    agent_id: String,
    #[serde(rename = "Encrypt")]
    encrypted_msg: String,
}
pub fn decrypt_message(
    key: &[u8],
    receiver_id: &str,
    token: &str,
    verify_info: &VerifyInfo,
    xml: &str,
) -> Result<CallbackMessage> {
    let encrypted_msg = quick_xml::de::from_str::<EncryptedXML>(xml)?.encrypted_msg;
    let sign = calc_signature(
        token,
        verify_info.timestamp.to_string().as_str(),
        verify_info.nonce.to_string().as_str(),
        &encrypted_msg,
    );
    if sign != verify_info.signature {
        return Err(anyhow!("签名不正确"));
    }
    // verify_message(token, verify_info, &encrypted_msg)?;

    let b = base64::engine::general_purpose::STANDARD.decode(encrypted_msg.as_bytes())?;
    let r = decrypt(key, &b)?;
    let (msg, decoded_receiver_id) = parse_plain_text(&r)?;
    if receiver_id != decoded_receiver_id {
        return Err(anyhow!("receiver_id={} 与服务端配置不一致", receiver_id));
    }
    Ok(decode_xml(&msg))
}

fn decode_xml(xml: &str) -> CallbackMessage {
    if let Ok(m) = quick_xml::de::from_str::<TextCallbackMessage>(xml) {
        return CallbackMessage::Text(m);
    }
    if let Ok(m) = quick_xml::de::from_str::<ImageCallbackMessage>(xml) {
        return CallbackMessage::Image(m);
    }
    CallbackMessage::Others
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename = "xml")]
pub struct TextCallbackMessage {
    #[serde(rename = "ToUserName")]
    pub to_user_name: String,
    #[serde(rename = "FromUserName")]
    pub from_user_name: String,
    #[serde(rename = "CreateTime")]
    pub create_time: i64,
    #[serde(rename = "MsgType")]
    pub msg_type: String,
    #[serde(rename = "Content")]
    pub content: String,
    #[serde(rename = "MsgId")]
    pub msg_id: String,
    #[serde(rename = "AgentID")]
    pub agent_id: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename = "xml")]
pub struct ImageCallbackMessage {
    #[serde(rename = "ToUserName")]
    pub to_user_name: String,
    #[serde(rename = "FromUserName")]
    pub from_user_name: String,
    #[serde(rename = "CreateTime")]
    pub create_time: i64,
    #[serde(rename = "MsgType")]
    pub msg_type: String,
    #[serde(rename = "PicUrl")]
    pub pic_url: String,
    #[serde(rename = "MediaId")]
    pub media_id: String,
    #[serde(rename = "MsgId")]
    pub msg_id: String,
    #[serde(rename = "AgentID")]
    pub agent_id: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum CallbackMessage {
    Text(TextCallbackMessage),
    Image(ImageCallbackMessage),
    Others,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::mp::callback::check_sign;
    use wechat_crypto::decode_aes_key;

    #[tokio::test]
    async fn test_check_sign() {
        let token = "QDG6eK";
        let receiver_id = "wx5823bf96d3bd56c7";
        let verify_msg_sign = "5c45ff5e21c57e6ad56bac8758b79b1d9ac89fd3";
        let verify_timestamp = 1409659589;
        let verify_nonce = 263014780;
        let verify_echo_str = "P9nAzCzyDtyTWESHep1vC5X9xho/qYX3Zpb4yKa9SKld1DsH3Iyt3tP3zNdtp+4RPcs8TgAE7OaBO+FZXvnaqQ==";
        let q = VerifyInfo {
            signature: verify_msg_sign.to_string(),
            timestamp: verify_timestamp,
            nonce: verify_nonce,
        };
        assert!(check_sign(token, &q, verify_echo_str,));
    }

    #[test]
    fn test_de() -> Result<()> {
        let xml = r#" <xml>
    <ToUserName><![CDATA[toUser]]></ToUserName>
    <AgentID><![CDATA[toAgentID]]></AgentID>
    <Encrypt><![CDATA[msg_encrypt]]></Encrypt></xml>"#;
        let exml: EncryptedXML = quick_xml::de::from_str(xml).unwrap();
        assert_eq!(exml.to_user_name, "toUser");
        assert_eq!(exml.agent_id, "toAgentID");
        assert_eq!(exml.encrypted_msg, "msg_encrypt");
        Ok(())
    }

    #[test]
    fn test_decrypt_message() -> Result<()> {
        let xml = "<xml><ToUserName><![CDATA[wx49f0ab532d5d035a]]></ToUserName>\n\
            <Encrypt><![CDATA[RgqEoJj5A4EMYlLvWO1F86ioRjZfaex/gePD0gOXTxpsq5Yj4GNglrBb8I2BAJVODGajiFnXBu7mCPatfjsu6IHCrsTyeDXzF6Bv283dGymzxh6ydJRvZsryDyZbLTE7rhnus50qGPMfp2wASFlzEgMW9z1ef/RD8XzaFYgm7iTdaXpXaG4+BiYyolBug/gYNx410cvkKR2/nPwBiT+P4hIiOAQqGp/TywZBtDh1yCF2KOd0gpiMZ5jSw3e29mTvmUHzkVQiMS6td7vXUaWOMZnYZlF3So2SjHnwh4jYFxdgpkHHqIrH/54SNdshoQgWYEvccTKe7FS709/5t6NMxuGhcUGAPOQipvWTT4dShyqio7mlsl5noTrb++x6En749zCpQVhDpbV6GDnTbcX2e8K9QaNWHp91eBdCRxthuL0=]]></Encrypt>\n\
            <AgentID><![CDATA[1]]></AgentID>\n\
            </xml>";
        let expected = CallbackMessage::Text(
            quick_xml::de::from_str::<TextCallbackMessage>(
                "<xml><ToUserName><![CDATA[wx49f0ab532d5d035a]]></ToUserName>\n\
            <FromUserName><![CDATA[messense]]></FromUserName>\n\
            <CreateTime>1411525903</CreateTime>\n\
            <MsgType><![CDATA[text]]></MsgType>\n\
            <Content><![CDATA[test]]></Content>\n\
            <MsgId>4363689963896700987</MsgId>\n\
            <AgentID>1</AgentID>\n\
            </xml>",
            )
            .unwrap(),
        );

        let aes_key = decode_aes_key("kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aQ")?;
        let decrypted = decrypt_message(
            &aes_key,
            "wx49f0ab532d5d035a",
            "123456",
            &VerifyInfo {
                signature: "74d92dfeb87ba7c714f89d98870ae5eb62dff26d".to_string(),
                timestamp: 1411525903,
                nonce: 461056294,
            },
            xml,
        )?;
        assert_eq!(&expected, &decrypted);
        dbg!(decrypted);
        Ok(())
    }

    #[test]
    fn test_text() -> Result<()> {
        let a = CallbackMessage::Text(TextCallbackMessage {
            to_user_name: "tun".to_string(),
            from_user_name: "fun".to_string(),
            create_time: 111,
            msg_type: "mt".to_string(),
            content: "c".to_string(),
            msg_id: "mi".to_string(),
            agent_id: "ai".to_string(),
        });

        dbg!(quick_xml::se::to_string(&a).unwrap());

        let xml = r#"<xml>
   <ToUserName><![CDATA[toUser]]></ToUserName>
   <FromUserName><![CDATA[fromUser]]></FromUserName> 
   <CreateTime>1348831860</CreateTime>
   <MsgType><![CDATA[text]]></MsgType>
   <Content><![CDATA[this is a test]]></Content>
   <MsgId>1234567890123456</MsgId>
   <AgentID>1</AgentID>
</xml>"#;
        let msg = dbg!(quick_xml::de::from_str::<TextCallbackMessage>(xml).unwrap());
        Ok(())
    }

    #[test]
    fn test_image() -> Result<()> {
        let xml = r#"<xml>
    <ToUserName><![CDATA[toUser]]></ToUserName>
    <FromUserName><![CDATA[fromUser]]></FromUserName>
    <CreateTime>1348831860</CreateTime>
    <MsgType><![CDATA[image]]></MsgType>
    <PicUrl><![CDATA[this is a url]]></PicUrl>
    <MediaId><![CDATA[media_id]]></MediaId>
    <MsgId>1234567890123456</MsgId>
    <AgentID>1</AgentID></xml>"#;
        let msg = dbg!(quick_xml::de::from_str::<ImageCallbackMessage>(xml).unwrap());
        Ok(())
    }
}

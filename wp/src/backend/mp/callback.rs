use crate::backend::mp::crypt::{calc_signature, cbc_decrypt, parse_plain_text};
use anyhow::{anyhow, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};

pub fn check_sign(sign: &str, token: &str, ts: i64, nonce: i64, data: &str) -> bool {
    let s = calc_signature(
        token,
        ts.to_string().as_str(),
        nonce.to_string().as_str(),
        data,
    );
    s == sign
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
    // verify_info: &VerifyInfo,
    xml: &str,
) -> Result<String> {
    let encrypted_msg = quick_xml::de::from_str::<EncryptedXML>(xml)?.encrypted_msg;
    // verify_message(token, verify_info, &encrypted_msg)?;
    let b = base64::engine::general_purpose::STANDARD.decode(encrypted_msg.as_bytes())?;
    let r = cbc_decrypt(key, &b)?;
    let (msg, decoded_receiver_id) = parse_plain_text(&r)?;
    if receiver_id != decoded_receiver_id {
        return Err(anyhow!("receiver_id={} 与服务端配置不一致", receiver_id));
    }
    Ok(msg)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::mp::callback::check_sign;
    use crate::backend::mp::crypt::{calc_signature, decode_aes_key};

    #[tokio::test]
    async fn test_check_sign() {
        let token = "QDG6eK";
        let receiver_id = "wx5823bf96d3bd56c7";
        let verify_msg_sign = "5c45ff5e21c57e6ad56bac8758b79b1d9ac89fd3";
        let verify_timestamp = 1409659589;
        let verify_nonce = 263014780;
        let verify_echo_str = "P9nAzCzyDtyTWESHep1vC5X9xho/qYX3Zpb4yKa9SKld1DsH3Iyt3tP3zNdtp+4RPcs8TgAE7OaBO+FZXvnaqQ==";
        assert!(check_sign(
            verify_msg_sign,
            token,
            verify_timestamp,
            verify_nonce,
            verify_echo_str,
        ));
    }

    #[test]
    fn test_de() -> Result<()> {
        let xml = r#" <xml>
    <ToUserName><![CDATA[toUser]]></ToUserName>
    <AgentID><![CDATA[toAgentID]]></AgentID>
    <Encrypt><![CDATA[msg_encrypt]]></Encrypt>
 </xml>"#;
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
        let expected = "<xml><ToUserName><![CDATA[wx49f0ab532d5d035a]]></ToUserName>\n\
            <FromUserName><![CDATA[messense]]></FromUserName>\n\
            <CreateTime>1411525903</CreateTime>\n\
            <MsgType><![CDATA[text]]></MsgType>\n\
            <Content><![CDATA[test]]></Content>\n\
            <MsgId>4363689963896700987</MsgId>\n\
            <AgentID>1</AgentID>\n\
            </xml>";

        // let signature = "6c729cc5480fab0c2e594b7e25a93d2dbef6ab97";
        // let timestamp = 1411525903;
        // let nonce = "461056294";
        // let config = WechatConfig::new(
        //     WechatConfig::decode_aes_key(&"kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aQ=".into())
        //         .unwrap(),
        //     "wx49f0ab532d5d035a".into(),
        //     "".into(),
        //     "123456".into(),
        // );
        // let verify_info = VerifyInfo {
        //     signature: signature.into(),
        //     timestamp,
        //     nonce: nonce.into(),
        //     msg_signature: Some("74d92dfeb87ba7c714f89d98870ae5eb62dff26d".into()),
        //     encrypt_type: Some("aes".into()),
        // };
        let aes_key = decode_aes_key("kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aQ")?;
        let decrypted = decrypt_message(&aes_key, "wx49f0ab532d5d035a", xml)?;
        assert_eq!(expected, &decrypted);
        Ok(())
    }
}

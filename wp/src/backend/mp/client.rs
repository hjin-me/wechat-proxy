use anyhow::{anyhow, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
struct AccessTokenResp {
    errcode: isize,                   // `json:"errcode"`
    errmsg: String,                   // `json:"errmsg"`
    pub access_token: Option<String>, // `json:"access_token" validate:"required"`
    pub expires_in: Option<i64>,      // `json:"expires_in" validate:"required"`
}
pub async fn get_access_token(corp_id: &str, corp_secret: &str) -> Result<(String, i64)> {
    let r = reqwest::get(format!(
        "https://qyapi.weixin.qq.com/cgi-bin/gettoken?corpid={corp_id}&corpsecret={corp_secret}"
    ))
    .await?
    .json::<AccessTokenResp>()
    .await?;
    if r.errcode != 0 {
        return Err(anyhow!("errcode: {}, errmsg: {}", r.errcode, r.errmsg));
    }
    if let (Some(access_token), Some(expires_in)) = (r.access_token, r.expires_in) {
        return Ok((access_token, expires_in));
    }
    Err(anyhow!("access_token or expires_in is None"))
}

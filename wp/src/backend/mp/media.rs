use anyhow::{anyhow, Result};
use leptos::ev::click;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct UploadMediaResponse {
    #[serde(rename = "errcode")]
    err_code: i64,
    #[serde(rename = "errmsg")]
    err_msg: String,
    #[serde(default)]
    media_id: String,
    #[serde(default)]
    created_at: i64,
    #[serde(default, rename = "type")]
    media_type: String,
}
pub async fn media_upload(
    client: &reqwest::Client,
    media_type: &str,
    token: &str,
    b: &[u8],
) -> Result<String> {
    let api = format!(
        "https://qyapi.weixin.qq.com/cgi-bin/media/upload?access_token={}&type={}",
        token, media_type
    );
    let img = Part::bytes(b.to_owned())
        .file_name("qrcode.png")
        .mime_str("image/png")?;
    let f = Form::new();
    let f = f.part("media", img);

    let res = client
        .post(api)
        .multipart(f)
        .send()
        .await?
        .json::<UploadMediaResponse>()
        .await?;
    if res.err_code != 0 {
        return Err(anyhow!(
            "上传图片失败 error: [{}]{}",
            res.err_code,
            res.err_msg
        ));
    }
    Ok(res.media_id)
}

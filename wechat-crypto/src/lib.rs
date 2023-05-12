//! 解决企业微信数据解码解密时遇到的异常问题，以便正常解析内容
//!
//! 可以应用在以下场景
//!
//! * 企业微信回调接口签名验证和解密
//! * 企业微信通讯录导出数据解密
//!
//! ## Example
//! ```rust
//! use base64::Engine;
//! use base64::engine::general_purpose::STANDARD;
//! use wechat_crypto::{calc_signature, decode_aes_key, decrypt, parse_plain_text};
//!
//! let encoded_aes_key = "kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aQ";
//! // 解码 aes_key
//! let aes_key = decode_aes_key(encoded_aes_key).unwrap();
//!
//! // 解密数据收到的数据
//! let r = decrypt(
//!     &aes_key,
//!     &STANDARD
//!         .decode("9s4gMv99m88kKTh/H8IdkNiFGeG9pd7vNWl50fGRWXY=")
//!         .unwrap(),
//! )
//! .unwrap();
//! dbg!(String::from_utf8_lossy(&r).to_string());
//!
//! // 提取数据中的正文和 receiver_id
//! let (t, r) = parse_plain_text(&r).unwrap();
//! assert_eq!("test", &t);
//!
//! // 签名验证
//! let token = "QDG6eK";
//! let verify_msg_sign = "5c45ff5e21c57e6ad56bac8758b79b1d9ac89fd3";
//! let verify_timestamp = "1409659589";
//! let verify_nonce = "263014780";
//! let verify_echo_str = "P9nAzCzyDtyTWESHep1vC5X9xho/qYX3Zpb4yKa9SKld1DsH3Iyt3tP3zNdtp+4RPcs8TgAE7OaBO+FZXvnaqQ==";
//!
//! // 验证签名是否匹配
//! assert_eq!(
//!     verify_msg_sign,
//!     calc_signature(token, verify_timestamp, verify_nonce, verify_echo_str)
//! );
//!
//! ```
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use anyhow::{anyhow, Result};
use base64::alphabet::STANDARD;
use base64::engine::{GeneralPurpose, GeneralPurposeConfig};
use base64::Engine;
use byteorder::{BigEndian, WriteBytesExt};
use cbc::cipher::block_padding::NoPadding;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::iter::repeat_with;

/// 验证签名的必须参数，该参数从 URL 获取
#[derive(Deserialize, Serialize, Debug)]
pub struct VerifyInfo {
    /// 企业微信签名，msg_signature
    #[serde(rename = "msg_signature")]
    pub signature: String,
    /// 时间戳 timestamp
    pub timestamp: i64,
    /// 随机数
    pub nonce: i64,
}
/// 解决企业微信 base64 数据 padding 问题
const G: GeneralPurpose = GeneralPurpose::new(
    &STANDARD,
    GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true),
);

/// 对原始的 encoded_aes_key 进行解码
pub fn decode_aes_key(encoded_aes_key: &str) -> Result<Vec<u8>> {
    Ok(G.decode(format!("{}=", encoded_aes_key))?)
}

/// 计算签名函数
/// ```rust
/// use base64::Engine;
/// use base64::engine::general_purpose::STANDARD;
/// use wechat_crypto::{calc_signature, decode_aes_key, decrypt, parse_plain_text};
/// fn test_calc_signature() {
///     let token = "QDG6eK";
///     let receiver_id = "wx5823bf96d3bd56c7";
///     let encoded_aes_key = "jWmYm7qr5nMoAUwZRjGtBxmz3KA1tkAj3ykkR6q2B2C";
///     let verify_msg_sign = "5c45ff5e21c57e6ad56bac8758b79b1d9ac89fd3";
///     let verify_timestamp = "1409659589";
///     let verify_nonce = "263014780";
///     let verify_echo_str = "P9nAzCzyDtyTWESHep1vC5X9xho/qYX3Zpb4yKa9SKld1DsH3Iyt3tP3zNdtp+4RPcs8TgAE7OaBO+FZXvnaqQ==";
///     assert_eq!(
///         verify_msg_sign,
///         calc_signature(token, verify_timestamp, verify_nonce, verify_echo_str)
///     );
///     let v = STANDARD
///         .decode(verify_echo_str)
///         .unwrap();
///     let aes_key = decode_aes_key(encoded_aes_key).unwrap();
///     let r = decrypt(aes_key.as_slice(), v.as_slice()).unwrap();
///     let (m, r) = dbg!(parse_plain_text(&r).unwrap());
///     assert_eq!(r, receiver_id);
///     assert_eq!("1616140317555161061", m.as_str());
/// }
/// ```
pub fn calc_signature(token: &str, ts: &str, nonce: &str, data: &str) -> String {
    let mut sort_arr = vec![token, ts, nonce, data];
    sort_arr.sort();
    let mut buffer = String::new();
    for value in sort_arr {
        buffer.push_str(value);
    }

    let mut sha = Sha1::new();

    sha.update(buffer.as_bytes());
    let signature = sha.finalize();
    format!("{:x}", signature)
}

/// 企业微信回调接口验证逻辑
///
/// 请根据使用的 http 框架获取 url 参数，然后传入该函数，该函数使用本 crate 其他几个函数组合完成签名验证。
///
/// 该函数未验证时间戳区间，需要自行验证
/// ```rust
/// use base64::Engine;
/// use base64::engine::general_purpose::STANDARD;
/// use wechat_crypto::{calc_signature, decode_aes_key, decrypt, parse_plain_text, verify_url, VerifyInfo};
/// fn test_verify_url() -> anyhow::Result<()> {
///     let token = "QDG6eK";
///     let receiver_id = "wx5823bf96d3bd56c7";
///     let encoded_aes_key = "jWmYm7qr5nMoAUwZRjGtBxmz3KA1tkAj3ykkR6q2B2C";
///     let aes_key = decode_aes_key(encoded_aes_key).unwrap();
///
///     let verify_msg_sign = "5c45ff5e21c57e6ad56bac8758b79b1d9ac89fd3";
///     let verify_timestamp = 1409659589;
///     let verify_nonce = 263014780;
///     let verify_echo_str = "P9nAzCzyDtyTWESHep1vC5X9xho/qYX3Zpb4yKa9SKld1DsH3Iyt3tP3zNdtp+4RPcs8TgAE7OaBO+FZXvnaqQ==";
///
///     let echo_str = verify_url(
///         token,
///         &VerifyInfo {
///             signature: verify_msg_sign.to_string(),
///             timestamp: verify_timestamp,
///             nonce: verify_nonce,
///         },
///         verify_echo_str,
///         aes_key.as_slice(),
///         receiver_id,
///     )?;
///     assert_eq!("1616140317555161061", echo_str.as_str());
///     Ok(())
/// }
/// ```
pub fn verify_url(
    token: &str,
    q: &VerifyInfo,
    echo_str: &str,
    aes_key: &[u8],
    corp_id: &str,
) -> Result<String> {
    let signature = calc_signature(
        token,
        q.timestamp.to_string().as_str(),
        q.nonce.to_string().as_str(),
        echo_str,
    );
    if signature != q.signature {
        return Err(anyhow::anyhow!("签名不正确"));
    }
    let es = base64::engine::general_purpose::STANDARD
        .decode(echo_str)
        .map_err(|e| anyhow::Error::new(e).context("echo_str base64 解密失败"))?;
    let plaintext = decrypt(aes_key, &es)?;
    let (msg, receiver_id) = parse_plain_text(&plaintext)?;
    if receiver_id != corp_id {
        return Err(anyhow!("receiver_id={} 与服务端配置不一致", receiver_id));
    }
    Ok(msg)
}

/// 对解密后的数据进行还原
///
/// 移除前16位随机数，返回消息体和消息的 receiver_id
pub fn parse_plain_text(plaintext: &[u8]) -> Result<(String, String)> {
    // let random = &plaintext[..16];
    let msg_len = u32::from_be_bytes([plaintext[16], plaintext[17], plaintext[18], plaintext[19]]);
    let msg = &plaintext[20..(20 + msg_len as usize)];
    let receiver_id = &plaintext[(20 + msg_len as usize)..];
    Ok((
        String::from_utf8_lossy(msg).to_string(),
        String::from_utf8_lossy(receiver_id).to_string(),
    ))
}

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
/// 使用 AES256 CBC 解密，解决了 PKCS7 填充问题
/// ```rust
/// use wechat_crypto::{decode_aes_key, decrypt, parse_plain_text};
/// use base64::Engine;
/// use base64::engine::general_purpose::STANDARD;
/// fn test_decrypt() {
///     let encoded_aes_key = "kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aQ";
///     let aes_key = decode_aes_key(encoded_aes_key).unwrap();
///     let r = decrypt(
///         aes_key.as_slice(),
///         &STANDARD
///             .decode("9s4gMv99m88kKTh/H8IdkNiFGeG9pd7vNWl50fGRWXY=")
///             .unwrap(),
///     )
///     .unwrap();
///     dbg!(String::from_utf8(r.clone()).unwrap());
///     let (t, _) = parse_plain_text(&r).unwrap();
///     assert_eq!("test", &t);
/// }
/// ```
pub fn decrypt(aes_key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let iv = &aes_key[..16];
    let key = &aes_key[..32];

    let cipher = Aes256CbcDec::new_from_slices(key, iv)
        .map_err(|e| anyhow::Error::new(e).context("初始化解密函数失败"))?;
    let mut buffer = vec![0u8; data.len()];

    let r = cipher
        .decrypt_padded_b2b_mut::<NoPadding>(data, &mut buffer)
        .map_err(|e| anyhow!("解密失败 {}", e))?;
    let end = r.len() - (r[r.len() - 1] as usize);
    Ok(r[..end].to_vec())
}

/// 使用 AES256 CBC 按照微信文档数据格式进行加密
/// ```rust
/// use base64::Engine;
/// use base64::engine::general_purpose::STANDARD;
/// use wechat_crypto::{decode_aes_key, encrypt};
/// fn test_encrypt() -> anyhow::Result<()> {
///     let encoded_aes_key = "kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aQ";
///     let aes_key = decode_aes_key(encoded_aes_key)?;
///     let encrypted = encrypt(aes_key.as_slice(), "test", "rust").unwrap();
///     assert_eq!(
///         "9s4gMv99m88kKTh/H8IdkNiFGeG9pd7vNWl50fGRWXY=",
///         &STANDARD.encode(encrypted)
///     );
///     Ok(())
/// }
/// ```
pub fn encrypt(aes_key: &[u8], plaintext: &str, corp_id: &str) -> Result<Vec<u8>> {
    let mut wtr = gen_random_byte();
    wtr.write_u32::<BigEndian>(plaintext.len() as u32)
        .map_err(|e| anyhow::Error::new(e).context("写入数据长度失败"))?;
    wtr.extend(plaintext.bytes());
    wtr.extend(corp_id.bytes());

    let iv = &aes_key[..16];
    let key = &aes_key[..32];

    let cipher = Aes256CbcEnc::new_from_slices(key, iv)
        .map_err(|e| anyhow::Error::new(e).context("初始化加密函数失败"))?;

    let mut buffer = vec![0u8; (wtr.len() + 15) / 16 * 16];
    let r = cipher
        .encrypt_padded_b2b_mut::<Pkcs7>(wtr.as_slice(), &mut buffer)
        .map_err(|e| anyhow!("解密失败 {}", e))?;
    Ok(r.to_vec())
}

fn gen_random_byte() -> Vec<u8> {
    if cfg!(test) {
        vec![
            49u8, 50, 51, 52, 53, 54, 55, 56, 57, 48, 49, 50, 51, 52, 53, 54,
        ]
    } else {
        repeat_with(|| fastrand::u8(..)).take(16).collect()
    }
}
#[cfg(test)]
mod test {
    use super::*;

    use base64::Engine;

    #[test]
    fn test_calc_signature() {
        let token = "QDG6eK";
        let receiver_id = "wx5823bf96d3bd56c7";
        let encoded_aes_key = "jWmYm7qr5nMoAUwZRjGtBxmz3KA1tkAj3ykkR6q2B2C";
        let verify_msg_sign = "5c45ff5e21c57e6ad56bac8758b79b1d9ac89fd3";
        let verify_timestamp = "1409659589";
        let verify_nonce = "263014780";
        let verify_echo_str = "P9nAzCzyDtyTWESHep1vC5X9xho/qYX3Zpb4yKa9SKld1DsH3Iyt3tP3zNdtp+4RPcs8TgAE7OaBO+FZXvnaqQ==";
        // 	echoStr, cryptErr := wxcpt.VerifyURL(verify_msg_sign, verify_timestamp, verify_nonce, verify_echo_str)
        assert_eq!(
            verify_msg_sign,
            calc_signature(token, verify_timestamp, verify_nonce, verify_echo_str)
        );
        let v = base64::engine::general_purpose::STANDARD
            .decode(verify_echo_str)
            .unwrap();
        let aes_key = decode_aes_key(encoded_aes_key).unwrap();
        let r = decrypt(aes_key.as_slice(), v.as_slice()).unwrap();
        let (m, r) = dbg!(parse_plain_text(&r).unwrap());
        assert_eq!(r, receiver_id);
        assert_eq!("1616140317555161061", m.as_str());

        let token = "QDG6eK";
        let signature = "477715d11cdb4164915debcba66cb864d751f3e6";
        let timestamps = "1409659813";
        let nonce = "1372623149";
        let msg_encrypt = "RypEvHKD8QQKFhvQ6QleEB4J58tiPdvo+rtK1I9qca6aM/wvqnLSV5zEPeusUiX5L5X/0lWfrf0QADHHhGd3QczcdCUpj911L3vg3W/sYYvuJTs3TUUkSUXxaccAS0qhxchrRYt66wiSpGLYL42aM6A8dTT+6k4aSknmPj48kzJs8qLjvd4Xgpue06DOdnLxAUHzM6+kDZ+HMZfJYuR+LtwGc2hgf5gsijff0ekUNXZiqATP7PF5mZxZ3Izoun1s4zG4LUMnvw2r+KqCKIw+3IQH03v+BCA9nMELNqbSf6tiWSrXJB3LAVGUcallcrw8V2t9EL4EhzJWrQUax5wLVMNS0+rUPA3k22Ncx4XXZS9o0MBH27Bo6BpNelZpS+/uh9KsNlY6bHCmJU9p8g7m3fVKn28H3KDYA5Pl/T8Z1ptDAVe0lXdQ2YoyyH2uyPIGHBZZIs2pDBS8R07+qN+E7Q==";

        assert_eq!(
            signature,
            calc_signature(token, timestamps, nonce, msg_encrypt)
        );

        let v = base64::engine::general_purpose::STANDARD
            .decode(msg_encrypt)
            .unwrap();

        dbg!(parse_plain_text(&decrypt(aes_key.as_slice(), v.as_slice()).unwrap()).unwrap());
        let signature = calc_signature("test", "123456", "test", "rust");
        assert_eq!("d6056f2bb3ad3e30f4afa5ef90cc9ddcdc7b7b27", signature);

        let receiver_id = "wx49f0ab532d5d035a";
        let encoded_aes_key = "kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aQ";
        let verify_echo_str = "4ByGGj+sVCYcvGeQYhaKIk1o0pQRNbRjxybjTGblXrBaXlTXeOo1+bXFXDQQb1o6co6Yh9Bv41n7hOchLF6p+Q==";

        let v = base64::engine::general_purpose::STANDARD
            .decode(verify_echo_str)
            .unwrap();
        let aes_key = decode_aes_key(encoded_aes_key).unwrap();
        let r = decrypt(aes_key.as_slice(), v.as_slice()).unwrap();
        let (m, r) = dbg!(parse_plain_text(&r).unwrap());
        assert_eq!(r, receiver_id);
        assert_eq!("5927782489442352469", m.as_str());
    }

    #[test]
    fn test_decode_aes_key() -> Result<()> {
        let encoded_aes_key = "IJUiXNpvGbODwKEBSEsAeOAPAhkqHqNCF6g19t9wfg2";
        let b = decode_aes_key(encoded_aes_key)?;
        let a = [
            32u8, 149, 34, 92, 218, 111, 25, 179, 131, 192, 161, 1, 72, 75, 0, 120, 224, 15, 2, 25,
            42, 30, 163, 66, 23, 168, 53, 246, 223, 112, 126, 13,
        ];
        assert_eq!(a, b.as_slice());
        Ok(())
    }

    #[test]
    fn test_decrypt() {
        let encoded_aes_key = "kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aQ";
        let aes_key = decode_aes_key(encoded_aes_key).unwrap();
        let r = decrypt(
            aes_key.as_slice(),
            &base64::engine::general_purpose::STANDARD
                .decode("9s4gMv99m88kKTh/H8IdkNiFGeG9pd7vNWl50fGRWXY=")
                .unwrap(),
        )
        .unwrap();
        dbg!(String::from_utf8(r.clone()).unwrap());
        let (t, _) = parse_plain_text(&r).unwrap();
        assert_eq!("test", &t);
    }

    #[test]
    fn test_encrypt() -> Result<()> {
        let encoded_aes_key = "kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aQ";
        let aes_key = decode_aes_key(encoded_aes_key)?;
        let encrypted = encrypt(aes_key.as_slice(), "test", "rust").unwrap();
        assert_eq!(
            "9s4gMv99m88kKTh/H8IdkNiFGeG9pd7vNWl50fGRWXY=",
            &base64::engine::general_purpose::STANDARD.encode(encrypted)
        );
        Ok(())
    }

    #[test]
    fn test_verify_url() -> Result<()> {
        let token = "QDG6eK";
        let receiver_id = "wx5823bf96d3bd56c7";
        let encoded_aes_key = "jWmYm7qr5nMoAUwZRjGtBxmz3KA1tkAj3ykkR6q2B2C";
        let aes_key = decode_aes_key(encoded_aes_key).unwrap();

        let verify_msg_sign = "5c45ff5e21c57e6ad56bac8758b79b1d9ac89fd3";
        let verify_timestamp = 1409659589;
        let verify_nonce = 263014780;
        let verify_echo_str = "P9nAzCzyDtyTWESHep1vC5X9xho/qYX3Zpb4yKa9SKld1DsH3Iyt3tP3zNdtp+4RPcs8TgAE7OaBO+FZXvnaqQ==";

        let echo_str = verify_url(
            token,
            &VerifyInfo {
                signature: verify_msg_sign.to_string(),
                timestamp: verify_timestamp,
                nonce: verify_nonce,
            },
            verify_echo_str,
            aes_key.as_slice(),
            receiver_id,
        )?;
        assert_eq!("1616140317555161061", echo_str.as_str());
        Ok(())
    }
}

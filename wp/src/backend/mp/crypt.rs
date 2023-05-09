use anyhow::{anyhow, Result};
use base64::Engine;
use sha1::Digest;
pub fn decode_aes_key(encoded_aes_key: &str) -> Result<Vec<u8>> {
    Ok(base64_decode(&format!("{}=", encoded_aes_key))?)
}

// 计算签名
pub fn calc_signature(token: &str, ts: &str, nonce: &str, data: &str) -> String {
    let mut sort_arr = vec![token, ts, nonce, data];
    sort_arr.sort();
    let mut buffer = String::new();
    for value in sort_arr {
        buffer.push_str(value);
    }

    // dbg!(buffer.as_str());
    let mut sha = sha1::Sha1::new();

    sha.update(buffer.as_bytes());
    let signature = sha.finalize();
    format!("{:x}", signature)
}

pub fn verify_url(
    token: &str,
    msg_signature: &str,
    timestamp: &str,
    nonce: &str,
    echo_str: &str,
    aes_key: &[u8],
    corp_id: &str,
) -> Result<String> {
    let signature = calc_signature(&token, &timestamp, &nonce, &echo_str);
    if signature != msg_signature {
        return Err(anyhow::anyhow!("签名不正确"));
    }
    let es = base64::engine::general_purpose::STANDARD
        .decode(&echo_str)
        .map_err(|e| anyhow::Error::new(e).context("echo_str base64 解密失败"))?;
    let plaintext = cbc_decrypt(aes_key, &es)?;
    let (msg, receiver_id) = parse_plain_text(&plaintext)?;
    if receiver_id != corp_id {
        return Err(anyhow!("receiver_id={} 与服务端配置不一致", receiver_id));
    }
    Ok(msg)
}

pub fn parse_plain_text(plaintext: &[u8]) -> Result<(String, String)> {
    let random = &plaintext[..16];
    debug!("random {:?}", random);
    let msg_len = u32::from_be_bytes([plaintext[16], plaintext[17], plaintext[18], plaintext[19]]);
    debug!("msg_len {:?}", msg_len);
    let msg = &plaintext[20..(20 + msg_len as usize)];
    let receiver_id = &plaintext[(20 + msg_len as usize)..];
    Ok((
        String::from_utf8_lossy(msg).to_string(),
        String::from_utf8_lossy(receiver_id).to_string(),
    ))
}

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use base64::alphabet::STANDARD;
use base64::engine::general_purpose::PAD;
use base64::engine::{GeneralPurpose, GeneralPurposeConfig};
use byteorder::{BigEndian, NativeEndian, WriteBytesExt};
use hex_literal::hex;
use rand::Rng;
use tracing::debug;

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
// aes decrypt with cbc
pub fn cbc_decrypt(aes_key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let iv = &aes_key[..16];
    let key = &aes_key[..32];

    let mut cipher = Aes256CbcDec::new_from_slices(key, iv)
        .map_err(|e| anyhow::Error::new(e).context("初始化解密函数失败"))?;
    let mut buffer = vec![0u8; data.len()];

    let r = cipher
        .decrypt_padded_b2b_mut::<Pkcs7>(data, &mut buffer)
        .map_err(|e| anyhow!("解密失败 {}", e))?;
    Ok(r.to_vec())
}

pub fn cbc_encrypt(aes_key: &[u8], plaintext: &str, corp_id: &str) -> Result<Vec<u8>, String> {
    let mut wtr = get_random_string().into_bytes();
    wtr.write_u32::<BigEndian>((plaintext.len() as u32))
        .map_err(|e| format!("write_u32: {}", e.to_string()))?;
    wtr.extend(plaintext.bytes());
    wtr.extend(corp_id.bytes());

    let iv = &aes_key[..16];
    let key = &aes_key[..32];

    let mut cipher = Aes256CbcEnc::new_from_slices(key, iv)
        .map_err(|e| format!("enc new_from_slices {}", e.to_string()))?;

    let mut buffer = vec![0u8; (wtr.len() + 15) / 16 * 16];
    let r = cipher
        .encrypt_padded_b2b_mut::<Pkcs7>(wtr.as_slice(), &mut buffer)
        .map_err(|e| format!("encrypt: {}", e.to_string()))?;
    Ok(r.to_vec())
}
const G: GeneralPurpose = GeneralPurpose::new(
    &STANDARD,
    GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true),
);
fn base64_decode(b: &str) -> Result<Vec<u8>> {
    Ok(G.decode(b)?)
}
fn get_random_string() -> String {
    if cfg!(test) {
        "1234567890123456".to_owned()
    } else {
        use rand::distributions::Alphanumeric;
        String::from_utf8(
            rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .collect(),
        )
        .unwrap()
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use anyhow::anyhow;
    use base64::Engine;
    use std::fs::File;
    use std::io::Read;
    use std::{env, fs};

    #[test]
    fn test_calc_signature() {
        let token = "QDG6eK";
        let receiver_id = "wx5823bf96d3bd56c7";
        let encoding_aes_key = "jWmYm7qr5nMoAUwZRjGtBxmz3KA1tkAj3ykkR6q2B2C";
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
        let aes_key = base64_decode(&format!("{}=", encoding_aes_key)).unwrap();
        let r = cbc_decrypt(aes_key.as_slice(), v.as_slice()).unwrap();
        let (m, r) = dbg!(parse_plain_text(&r).unwrap());
        assert_eq!(r, receiver_id);
        assert_eq!("1616140317555161061", m.as_str());

        let token = "QDG6eK";
        let signature = "477715d11cdb4164915debcba66cb864d751f3e6";
        let timestamps = "1409659813";
        let nonce = "1372623149";
        let encoding_aes_key = "jWmYm7qr5nMoAUwZRjGtBxmz3KA1tkAj3ykkR6q2B2C";
        let msg_encrypt = "RypEvHKD8QQKFhvQ6QleEB4J58tiPdvo+rtK1I9qca6aM/wvqnLSV5zEPeusUiX5L5X/0lWfrf0QADHHhGd3QczcdCUpj911L3vg3W/sYYvuJTs3TUUkSUXxaccAS0qhxchrRYt66wiSpGLYL42aM6A8dTT+6k4aSknmPj48kzJs8qLjvd4Xgpue06DOdnLxAUHzM6+kDZ+HMZfJYuR+LtwGc2hgf5gsijff0ekUNXZiqATP7PF5mZxZ3Izoun1s4zG4LUMnvw2r+KqCKIw+3IQH03v+BCA9nMELNqbSf6tiWSrXJB3LAVGUcallcrw8V2t9EL4EhzJWrQUax5wLVMNS0+rUPA3k22Ncx4XXZS9o0MBH27Bo6BpNelZpS+/uh9KsNlY6bHCmJU9p8g7m3fVKn28H3KDYA5Pl/T8Z1ptDAVe0lXdQ2YoyyH2uyPIGHBZZIs2pDBS8R07+qN+E7Q==";

        assert_eq!(
            signature,
            calc_signature(token, timestamps, nonce, msg_encrypt)
        );

        let v = base64::engine::general_purpose::STANDARD
            .decode(msg_encrypt)
            .unwrap();
        let signature = calc_signature("test", "123456", "test", "rust");
        assert_eq!("d6056f2bb3ad3e30f4afa5ef90cc9ddcdc7b7b27", signature);

        let token = "QDG6eK";
        let receiver_id = "wx49f0ab532d5d035a";
        let encoding_aes_key = "kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aQ";
        let verify_msg_sign = "5c45ff5e21c57e6ad56bac8758b79b1d9ac89fd3";
        let verify_timestamp = "1411443780";
        let verify_nonce = "437374425";
        let verify_echo_str = "4ByGGj+sVCYcvGeQYhaKIk1o0pQRNbRjxybjTGblXrBaXlTXeOo1+bXFXDQQb1o6co6Yh9Bv41n7hOchLF6p+Q==";

        let v = base64::engine::general_purpose::STANDARD
            .decode(verify_echo_str)
            .unwrap();
        let aes_key = base64_decode(&format!("{}=", encoding_aes_key)).unwrap();
        let r = cbc_decrypt(aes_key.as_slice(), v.as_slice()).unwrap();
        let (m, r) = dbg!(parse_plain_text(&r).unwrap());
        assert_eq!(r, receiver_id);
        assert_eq!("5927782489442352469", m.as_str());
    }

    #[test]
    fn test_base64() -> Result<()> {
        let encoding_aes_key = "IJUiXNpvGbODwKEBSEsAeOAPAhkqHqNCF6g19t9wfg2";
        let b = base64_decode(&format!("{}=", encoding_aes_key))?;
        let a = [
            32u8, 149, 34, 92, 218, 111, 25, 179, 131, 192, 161, 1, 72, 75, 0, 120, 224, 15, 2, 25,
            42, 30, 163, 66, 23, 168, 53, 246, 223, 112, 126, 13,
        ];
        assert_eq!(a, b.as_slice());
        Ok(())
    }

    #[test]
    fn test_decrypt() {
        let encoding_aes_key = "kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aQ";
        let aes_key = base64_decode(&format!("{}=", encoding_aes_key)).unwrap();
        let r = cbc_decrypt(
            aes_key.as_slice(),
            &base64_decode("9s4gMv99m88kKTh/H8IdkNiFGeG9pd7vNWl50fGRWXY=").unwrap(),
        )
        .unwrap();
        dbg!(String::from_utf8(r.clone()).unwrap());
        let (t, r) = parse_plain_text(&r).unwrap();
        assert_eq!("test", &t);
    }

    #[test]
    fn test_encrypt() -> Result<()> {
        let encoding_aes_key = "kWxPEV2UEDyxWpmPdKC3F4dgPDmOvfKX1HGnEUDS1aQ";
        let aes_key = base64_decode(&format!("{}=", encoding_aes_key))?;
        let encrypted = cbc_encrypt(aes_key.as_slice(), "test", "rust").unwrap();
        assert_eq!(
            "9s4gMv99m88kKTh/H8IdkNiFGeG9pd7vNWl50fGRWXY=",
            &base64::engine::general_purpose::STANDARD.encode(encrypted)
        );
        Ok(())
    }
}

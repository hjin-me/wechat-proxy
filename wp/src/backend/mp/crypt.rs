use anyhow::Result;
use base64::Engine;
use sha1::Digest;

// 计算签名
fn calc_signature(token: &str, ts: &str, nonce: &str, data: &str) -> String {
    let mut sort_arr = vec![token, ts, nonce, data];
    sort_arr.sort();
    let mut buffer = String::new();
    for value in sort_arr {
        buffer.push_str(value);
    }
    let mut sha = sha1::Sha1::new();

    sha.update(buffer.as_bytes());
    let signature = sha.finalize();
    format!("{:x}", signature)
}

// func (self *WXBizMsgCrypt) VerifyURL(msg_signature, timestamp, nonce, echostr string) ([]byte, *CryptError) {
// 	signature := self.calSignature(timestamp, nonce, echostr)
//
// 	if strings.Compare(signature, msg_signature) != 0 {
// 		return nil, NewCryptError(ValidateSignatureError, "signature not equal")
// 	}
//
// 	plaintext, err := self.cbcDecrypter(echostr)
// 	if nil != err {
// 		return nil, err
// 	}
//
// 	_, _, msg, receiver_id, err := self.ParsePlainText(plaintext)
// 	if nil != err {
// 		return nil, err
// 	}
//
// 	if len(self.receiver_id) > 0 && strings.Compare(string(receiver_id), self.receiver_id) != 0 {
// 		fmt.Println(string(receiver_id), self.receiver_id, len(receiver_id), len(self.receiver_id))
// 		return nil, NewCryptError(ValidateCorpidError, "receiver_id is not equil")
// 	}
//
// 	return msg, nil
// }

pub fn verify_url(
    token: &str,
    msg_signature: &str,
    timestamp: &str,
    nonce: &str,
    echo_str: &str,
    encoding_aes_key: &str,
) -> Result<String, String> {
    let signature = calc_signature(&token, &timestamp, &nonce, &echo_str);
    if signature != msg_signature {
        return Err("signature not equal".to_string());
    }
    let plaintext = cbc_decrypter(&echo_str.as_bytes().to_vec(), &encoding_aes_key)?;
    let (msg, receiver_id) = parse_plain_text(&plaintext)?;
    if receiver_id != "" {
        return Err("receiver_id is not equil".to_string());
    }
    Ok(msg)
}

fn parse_plain_text(plaintext: &[u8]) -> Result<(String, String), String> {
    let random = &plaintext[..16];
    let msg_len = u32::from_be_bytes([plaintext[16], plaintext[17], plaintext[18], plaintext[19]]);
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
use hex_literal::hex;

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
// aes decrypt with cbc
fn cbc_decrypter(data: &Vec<u8>, encoding_aeskey: &str) -> Result<Vec<u8>, String> {
    let key = base64_decode(&format!("{}=", encoding_aeskey))
        .map_err(|e| format!("key base64decode: {}", e.to_string()))?;
    let iv = &key[..16];
    let key = &key[..32];
    // let mut data = base64::engine::general_purpose::STANDARD
    //     .decode(data)
    //     .map_err(|e| format!("base64 data: {}", e.to_string()))?;
    let data = data.clone();

    let mut cipher = Aes256CbcDec::new_from_slices(key, iv)
        .map_err(|e| format!("new_from_slices {}", e.to_string()))?;
    let mut buffer = vec![0u8; data.len()];

    let r = cipher
        .decrypt_padded_b2b_mut::<Pkcs7>(data.as_slice(), &mut buffer)
        .map_err(|e| format!("decrypt: {}", e.to_string()))?;
    Ok(r.to_vec())
}
const G: GeneralPurpose = GeneralPurpose::new(
    &STANDARD,
    GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true),
);
fn base64_decode(b: &str) -> Result<Vec<u8>> {
    Ok(G.decode(b)?)
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
        let r = cbc_decrypter(&v, encoding_aes_key).unwrap();
        let (m, r) = dbg!(parse_plain_text(&r).unwrap());
        assert_eq!(r, receiver_id);
        assert_eq!("1616140317555161061", m.as_str());

        let token = "QDG6eK";
        let signature = "477715d11cdb4164915debcba66cb864d751f3e6";
        let timestamps = "1409659813";
        let nonce = "1372623149";
        let msg_encrypt = "RypEvHKD8QQKFhvQ6QleEB4J58tiPdvo+rtK1I9qca6aM/wvqnLSV5zEPeusUiX5L5X/0lWfrf0QADHHhGd3QczcdCUpj911L3vg3W/sYYvuJTs3TUUkSUXxaccAS0qhxchrRYt66wiSpGLYL42aM6A8dTT+6k4aSknmPj48kzJs8qLjvd4Xgpue06DOdnLxAUHzM6+kDZ+HMZfJYuR+LtwGc2hgf5gsijff0ekUNXZiqATP7PF5mZxZ3Izoun1s4zG4LUMnvw2r+KqCKIw+3IQH03v+BCA9nMELNqbSf6tiWSrXJB3LAVGUcallcrw8V2t9EL4EhzJWrQUax5wLVMNS0+rUPA3k22Ncx4XXZS9o0MBH27Bo6BpNelZpS+/uh9KsNlY6bHCmJU9p8g7m3fVKn28H3KDYA5Pl/T8Z1ptDAVe0lXdQ2YoyyH2uyPIGHBZZIs2pDBS8R07+qN+E7Q==";
        let encoding_aes_key = "jWmYm7qr5nMoAUwZRjGtBxmz3KA1tkAj3ykkR6q2B2C";

        assert_eq!(
            signature,
            calc_signature(token, timestamps, nonce, msg_encrypt)
        );

        let v = base64::engine::general_purpose::STANDARD
            .decode(msg_encrypt)
            .unwrap();
        // dbg!(cbc_decrypter(&v, encoding_aes_key).unwrap());
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
    fn test_decode() -> Result<()> {
        let encoding_aes_key = "IJUiXNpvGbODwKEBSEsAeOAPAhkqHqNCF6g19t9wfg2";
        dbg!(env::current_dir()?);
        let b = fs::read("./src/backend/mp/data.json")?;
        let r = cbc_decrypter(&b, encoding_aes_key).unwrap();
        dbg!(String::from_utf8_lossy(&r));
        Ok(())
    }
}

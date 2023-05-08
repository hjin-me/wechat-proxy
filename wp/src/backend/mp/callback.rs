use crate::backend::mp::crypt::calc_signature;
use anyhow::Result;

pub fn check_sign(sign: &str, token: &str, ts: i64, nonce: i64, data: &str) -> bool {
    let s = calc_signature(
        token,
        ts.to_string().as_str(),
        nonce.to_string().as_str(),
        data,
    );
    s == sign
}

#[cfg(test)]
mod test {
    use crate::backend::mp::callback::check_sign;
    use crate::backend::mp::crypt::calc_signature;

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
}

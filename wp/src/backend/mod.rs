pub mod api;
pub mod chatglm;
pub mod context;
pub mod mp;
pub mod xx;

use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct Config {
    pub corp_id: String,
    pub corp_secret: String,
    pub agent_id: i64,
    pub encoded_aes_key: String,
    pub token: String,
    pub glm_api: String,
}

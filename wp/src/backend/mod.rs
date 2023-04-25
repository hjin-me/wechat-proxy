pub mod api;
pub mod mp;

use anyhow::Result;
use serde::Deserialize;
use tracing::{info, trace};
#[derive(Debug, Deserialize)]
pub struct Config {
    pub corp_id: String,
    pub corp_secret: String,
    pub agent_id: i64,
}

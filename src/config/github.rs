use std::time::Duration;

use serde::Deserialize;

use super::duration_human;

fn default_timeout() -> Duration {
    Duration::from_secs(15 * 60)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubConfig {
    pub app_id: u64,
    pub jwt_key_file: String,
    pub webhook_secret: String,
    #[serde(default = "default_timeout")]
    #[serde(deserialize_with = "duration_human::deserialize")]
    pub polling_interval: Duration,
}

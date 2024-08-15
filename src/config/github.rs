use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubConfig {
    pub app_id: u64,
    pub jwt_key_file: String,
}

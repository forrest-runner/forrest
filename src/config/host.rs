use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostConfig {
    pub base_dir: PathBuf,
}

use std::path::PathBuf;

use serde::Deserialize;

use super::size_in_bytes::SizeInBytes;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostConfig {
    pub base_dir: PathBuf,
    pub ram: SizeInBytes,
}

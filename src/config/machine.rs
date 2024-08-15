use std::collections::HashMap;

use serde::Deserialize;

use super::size_in_bytes::SizeInBytes;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineConfig {
    pub ram: SizeInBytes,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repository {
    pub machines: HashMap<String, MachineConfig>,
}

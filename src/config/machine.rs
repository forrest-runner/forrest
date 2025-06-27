use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

use super::size_in_bytes::SizeInBytes;
use crate::machines::Triplet;

#[derive(Deserialize)]
pub struct SetupTemplate {
    pub path: PathBuf,

    #[serde(default)]
    pub parameters: HashMap<String, String>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum SeedBasePolicy {
    IfNewer,
    Always,
    Never,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposedDirectory {
    pub path: PathBuf,
    pub tag: String,
    #[serde(default)]
    pub writable: bool,
}

fn default_artifact_name() -> String {
    "artifact".into()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    #[serde(default = "default_artifact_name")]
    pub name: String,
    pub path: String,
    pub url: String,
    pub quota: SizeInBytes,
    pub token: Option<String>,
}

impl Default for SeedBasePolicy {
    fn default() -> Self {
        Self::IfNewer
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineConfig {
    pub base_machine: Option<Triplet>,
    pub base_image: Option<PathBuf>,
    pub setup_template: SetupTemplate,

    #[serde(default)]
    pub use_base: SeedBasePolicy,

    pub cpus: u32,
    pub disk: SizeInBytes,
    pub ram: SizeInBytes,

    #[serde(default)]
    pub shared: Vec<ExposedDirectory>,

    #[serde(default)]
    pub artifacts: Vec<Artifact>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repository {
    pub persistence_token: Option<String>,
    pub machines: HashMap<String, MachineConfig>,
}

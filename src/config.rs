use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use log::{error, info};
use serde::Deserialize;

mod duration_human;
mod github;
mod host;
mod machine;
mod size_in_bytes;

pub use github::GitHubConfig;
pub use host::HostConfig;
pub use machine::{Artifact, MachineConfig, Repository, SeedBasePolicy};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    pub github: GitHubConfig,
    pub host: HostConfig,
    pub repositories: HashMap<String, HashMap<String, Repository>>,
}

struct Inner {
    path: PathBuf,
    config_file: Arc<ConfigFile>,
    last_modified: SystemTime,
}

#[derive(Clone)]
pub struct Config {
    inner: Arc<Mutex<Inner>>,
}

impl ConfigFile {
    fn from_file(fd: &mut File) -> serde_yaml_ng::Result<Arc<Self>> {
        // First we read the config file as generic serde_yaml_ng Value.
        let mut cfg: serde_yaml_ng::Value = serde_yaml_ng::from_reader(fd)?;

        // Then we apply merges / overrides like these:
        //
        // machine_snippets:
        //   small: &machine-small
        //     ram: 8G
        //     …
        //   large: &machine-large
        //     << : *machine-small
        //     ram: 32G
        //
        cfg.apply_merge()?;

        if let Some(cfg_mapping) = cfg.as_mapping_mut() {
            // Remove all top level fields from the config who's name ends
            // in `_snippets`.
            // This allows using keys like `machine_snippets` which do not
            // adhere to the syntax.

            cfg_mapping.retain(|k, _| {
                k.as_str()
                    .map(|k| !k.ends_with("_snippets"))
                    .unwrap_or(true)
            });
        }

        // And then we convert to our config format.
        let cfg = serde_yaml_ng::from_value(cfg)?;

        Ok(Arc::new(cfg))
    }
}

impl Inner {
    fn should_refresh(&self) -> Option<(File, SystemTime)> {
        let fd = match File::open(&self.path) {
            Ok(fd) => fd,
            Err(e) => {
                error!("Failed to open config file, will not refresh: {e}");
                return None;
            }
        };

        let modified = match fd.metadata().and_then(|m| m.modified()) {
            Ok(meta) => meta,
            Err(e) => {
                error!("Failed to check config file metadata, will not refresh: {e}");
                return None;
            }
        };

        (modified > self.last_modified).then_some((fd, modified))
    }

    fn get(&mut self) -> Arc<ConfigFile> {
        if let Some((mut fd, last_modified)) = self.should_refresh() {
            match ConfigFile::from_file(&mut fd) {
                Ok(cf) => {
                    self.config_file = cf;
                    self.last_modified = last_modified;
                    info!("Re-read config file {}", self.path.display());
                }
                Err(e) => {
                    error!("Failed to re-read config: {e}. Reusing previous version.");
                }
            }
        }

        self.config_file.clone()
    }
}

impl Config {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mut fd = File::open(&path)?;

        let config_file = ConfigFile::from_file(&mut fd)?;
        let last_modified = fd.metadata()?.modified()?;

        let inner = Inner {
            path: path.as_ref().into(),
            config_file,
            last_modified,
        };

        let inner = Arc::new(Mutex::new(inner));

        Ok(Config { inner })
    }

    /// Get the current configuration
    ///
    /// This will check if the file changed on disk and if so will try to
    /// re-read it.
    /// If reading or parsing fails it will log an error and keep using the
    /// old version.
    pub fn get(&self) -> Arc<ConfigFile> {
        self.inner.lock().unwrap().get()
    }
}

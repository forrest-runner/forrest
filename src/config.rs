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
pub use machine::{Artifact, MachineConfig, NetworkInterface, Repository, SeedBasePolicy};

#[derive(Debug, Deserialize, PartialEq)]
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

fn contains_merge(value: &yaml_serde::Value) -> bool {
    value
        .as_mapping()
        .map(|mapping| mapping.contains_key("<<") || mapping.values().any(contains_merge))
        .unwrap_or(false)
}

fn remove_dot_keys(mapping: &mut yaml_serde::Mapping) {
    // Remove all keys from the config that start with a dot.
    // This is similar to how e.g. gitlab CI handles reusable YAML snippets.
    mapping.retain(|k, _| k.as_str().map(|k| !k.starts_with(".")).unwrap_or(true));

    // Recursively walk through all mappings in the config and remove
    // dot prefixed keys there as well.
    mapping
        .values_mut()
        .filter_map(yaml_serde::Value::as_mapping_mut)
        .for_each(remove_dot_keys);
}

impl ConfigFile {
    fn from_reader<R>(reader: R) -> yaml_serde::Result<Arc<Self>>
    where
        R: std::io::Read,
    {
        // First we read the config file as generic yaml_serde Value.
        let mut cfg: yaml_serde::Value = yaml_serde::from_reader(reader)?;

        // Then we apply merges / overrides like these:
        //
        // .machines:
        //   small: &machine-small
        //     ram: 8G
        //     …
        //   large: &machine-large
        //     << : *machine-small
        //     ram: 32G
        //
        // We may need to do this multiple times, because `apply_merge` does
        // not resolve nested merges by itself.
        while contains_merge(&cfg) {
            cfg.apply_merge()?;
        }

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

            // Recursively walk through all mappings in the config and remove
            // dot prefixed keys.
            remove_dot_keys(cfg_mapping);
        }

        // And then we convert to our config format.
        let cfg = yaml_serde::from_value(cfg)?;

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
            match ConfigFile::from_reader(&mut fd) {
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

        let config_file = ConfigFile::from_reader(&mut fd)?;
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

#[cfg(test)]
mod tests {
    use super::ConfigFile;

    const CONFIG_NESTED: &[u8] = br#"
        host:
          base_dir: /srv/forrest
          ram: 120G

        github:
          app_id: 1234
          jwt_key_file: key.pem
          polling_interval: 15m
          webhook_secret: Some super secret text

        .machines:
          machine-small: &machine-small
            setup_template:
              path: /etc/forrest/templates/generic
              parameters:
                RUNNER_VERSION: "2.318.0"
                RUNNER_HASH: "28ed88e4cedf0fc93201a901e392a70463dbd0213f2ce9d57a4ab495027f3e2f"
            base_image: /srv/forrest/images/debian-12-generic-amd64.raw
            cpus: 4
            disk: 16G
            ram: 4G
          machine-medium: &machine-medium
            << : *machine-small
            cpus: 8
            disk: 32G
            ram: 8G

        repositories:
          hnez:
            forrest-images:
              persistence_token: <PERSISTENCE_TOKEN>
              machines:
                debian-base:
                  << : *machine-small
                  use_base: always
                debian-yocto:
                  << : *machine-small
                  base_machine: hnez/forrest-images/debian-base
                  use_base: always

            forrest-test:
              machines:
                test-debian:
                  << : *machine-medium
                  base_machine: hnez/forrest-images/debian-base
        "#;

    const CONFIG_FLAT: &[u8] = br#"
        host:
          base_dir: /srv/forrest
          ram: 120G

        github:
          app_id: 1234
          jwt_key_file: key.pem
          polling_interval: 15m
          webhook_secret: Some super secret text

        repositories:
          hnez:
            forrest-images:
              persistence_token: <PERSISTENCE_TOKEN>
              machines:
                debian-base:
                  setup_template:
                    path: /etc/forrest/templates/generic
                    parameters:
                      RUNNER_VERSION: "2.318.0"
                      RUNNER_HASH: "28ed88e4cedf0fc93201a901e392a70463dbd0213f2ce9d57a4ab495027f3e2f"
                  base_image: /srv/forrest/images/debian-12-generic-amd64.raw
                  cpus: 4
                  disk: 16G
                  ram: 4G
                  use_base: always
                debian-yocto:
                  setup_template:
                    path: /etc/forrest/templates/generic
                    parameters:
                      RUNNER_VERSION: "2.318.0"
                      RUNNER_HASH: "28ed88e4cedf0fc93201a901e392a70463dbd0213f2ce9d57a4ab495027f3e2f"
                  base_image: /srv/forrest/images/debian-12-generic-amd64.raw
                  cpus: 4
                  disk: 16G
                  ram: 4G
                  base_machine: hnez/forrest-images/debian-base
                  use_base: always

            forrest-test:
              machines:
                test-debian:
                  setup_template:
                    path: /etc/forrest/templates/generic
                    parameters:
                      RUNNER_VERSION: "2.318.0"
                      RUNNER_HASH: "28ed88e4cedf0fc93201a901e392a70463dbd0213f2ce9d57a4ab495027f3e2f"
                  base_image: /srv/forrest/images/debian-12-generic-amd64.raw
                  cpus: 8
                  disk: 32G
                  ram: 8G
                  base_machine: hnez/forrest-images/debian-base

        "#;

    #[test]
    fn nested_snippets() {
        let config_file_nested = ConfigFile::from_reader(CONFIG_NESTED).unwrap();
        let config_file_flat = ConfigFile::from_reader(CONFIG_FLAT).unwrap();

        assert_eq!(config_file_nested, config_file_flat);
    }
}

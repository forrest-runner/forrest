use std::path::{Path, PathBuf};

use anyhow::bail;
use serde::de::{Deserialize, Deserializer, Error};

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct OwnerAndRepo {
    owner: String,
    repository: String,
}

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct OwnerRepoLabels {
    owner: String,
    repository: String,
    labels: Vec<String>,
}

pub struct OwnerRepoMachine {
    owner: String,
    repository: String,
    machine_name: String,
}

impl OwnerAndRepo {
    pub fn new(owner: impl ToString, repository: impl ToString) -> Self {
        Self {
            owner: owner.to_string(),
            repository: repository.to_string(),
        }
    }

    pub fn into_orl(self, labels: Vec<String>) -> OwnerRepoLabels {
        OwnerRepoLabels {
            owner: self.owner,
            repository: self.repository,
            labels,
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repository(&self) -> &str {
        &self.repository
    }
}

impl std::fmt::Display for OwnerAndRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}/{}", self.owner, self.repository)
    }
}

impl OwnerRepoLabels {
    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repository(&self) -> &str {
        &self.repository
    }

    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    pub fn machine_name(&self) -> anyhow::Result<&str> {
        match self.labels.as_slice() {
            [self_hosted, forrest, machine_name] => {
                if self_hosted != "self-hosted" {
                    bail!("First of three labels is not \"self-hosted\"");
                }

                if forrest != "forrest" {
                    bail!("Second of three labels is not \"forrest\"");
                }

                Ok(machine_name)
            }
            _ => {
                bail!(
                    "Job has unsupported number of labels: {}",
                    self.labels.len()
                );
            }
        }
    }

    pub fn into_owner_repo_machine(self) -> anyhow::Result<OwnerRepoMachine> {
        let machine_name = self.machine_name()?.to_owned();

        let orm = OwnerRepoMachine {
            owner: self.owner,
            repository: self.repository,
            machine_name,
        };

        Ok(orm)
    }

    pub fn into_owner_and_repo(self) -> OwnerAndRepo {
        OwnerAndRepo {
            owner: self.owner,
            repository: self.repository,
        }
    }
}

impl std::fmt::Display for OwnerRepoLabels {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Normal runs-on format:
        //   runs-on: [self-hosted, forrest, machine]
        //   "owner repo [self-hosted, forrest, machine]"

        write!(f, "{} {} [", self.owner, self.repository)?;

        let nl = self.labels.len();

        for i in 0..nl {
            write!(f, "{}", self.labels[i])?;

            if i < (nl - 1) {
                // Do not print a trailing comma
                write!(f, ", ")?;
            }
        }

        write!(f, "]")?;

        Ok(())
    }
}

impl std::fmt::Debug for OwnerRepoLabels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl OwnerRepoMachine {
    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn repository(&self) -> &str {
        &self.repository
    }

    pub fn machine_name(&self) -> &str {
        &self.machine_name
    }

    pub(super) fn run_dir_path(&self, base_dir_path: &Path, runner_name: &str) -> PathBuf {
        base_dir_path
            .join("runs")
            .join(&self.owner)
            .join(&self.repository)
            .join(&self.machine_name)
            .join(runner_name)
    }

    pub(super) fn machine_image_path(&self, base_dir_path: &Path) -> PathBuf {
        base_dir_path
            .join("machines")
            .join(&self.owner)
            .join(&self.repository)
            .join(format!("{}.img", self.machine_name))
    }
}

impl std::fmt::Display for OwnerRepoMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}",
            self.owner, self.repository, self.machine_name
        )
    }
}

impl<'de> Deserialize<'de> for OwnerRepoMachine {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let orm_str: String = Deserialize::deserialize(deserializer)?;

        let parts: Vec<&str> = orm_str.split('/').collect();
        let parts_len = parts.len();

        if parts_len != 3 {
            return Err(D::Error::invalid_length(
                parts_len,
                &"Expected string of format <user>/<repo>/<machine type>",
            ));
        }

        let orm = OwnerRepoMachine {
            owner: parts[0].to_owned(),
            repository: parts[1].to_owned(),
            machine_name: parts[2].to_owned(),
        };

        Ok(orm)
    }
}

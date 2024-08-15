use serde::de::{Deserialize, Deserializer, Error};

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct OwnerAndRepo {
    owner: String,
    repository: String,
}

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct Triplet {
    owner: String,
    repository: String,
    machine_name: String,
}

impl std::fmt::Display for OwnerAndRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}/{}", self.owner, self.repository)
    }
}

impl Triplet {
    pub fn new(
        owner: impl ToString,
        repository: impl ToString,
        machine_name: impl ToString,
    ) -> Self {
        Self {
            owner: owner.to_string(),
            repository: repository.to_string(),
            machine_name: machine_name.to_string(),
        }
    }
}

impl std::fmt::Display for Triplet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}",
            self.owner, self.repository, self.machine_name
        )
    }
}

impl std::fmt::Debug for Triplet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl<'de> Deserialize<'de> for Triplet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let triplet_str: String = Deserialize::deserialize(deserializer)?;

        let parts: Vec<&str> = triplet_str.split('/').collect();
        let parts_len = parts.len();

        if parts_len != 3 {
            return Err(D::Error::invalid_length(
                parts_len,
                &"Expected string of format <user>/<repo>/<machine type>",
            ));
        }

        Ok(Self::new(parts[0], parts[1], parts[2]))
    }
}

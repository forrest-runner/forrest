use serde::{Deserialize, Deserializer};

#[derive(Clone, Copy)]
pub struct SizeInBytes(u64);

impl<'de> Deserialize<'de> for SizeInBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut size_str: String = Deserialize::deserialize(deserializer)?;

        let multiplier = match size_str.pop() {
            Some('B') => 1,
            Some('K') => 1024,
            Some('M') => 1024 * 1024,
            Some('G') => 1024 * 1024 * 1024,
            Some('T') => 1024 * 1024 * 1024 * 1024,
            _ => panic!("Failed to parse size string '{size_str}': unknown unit"),
        };

        let size: u64 = size_str
            .parse()
            .expect("Failed to parse size string '{size_str}': can not parse as u64");

        Ok(SizeInBytes(size * multiplier))
    }
}

impl SizeInBytes {
    pub fn bytes(&self) -> u64 {
        self.0
    }

    pub fn kilobyes(self) -> u64 {
        self.bytes() / 1024
    }

    pub fn megabytes(&self) -> u64 {
        self.kilobyes() / 1024
    }
}

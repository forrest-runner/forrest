use std::time::Duration;

use serde::{Deserialize, Deserializer};

pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let mut duration_str: String = Deserialize::deserialize(deserializer)?;

    let unit = duration_str.pop();

    let multiplier = match unit {
        Some('s') => 1,
        Some('m') => 60,
        Some('h') => 60 * 60,
        Some('d') => 24 * 60 * 60,
        _ => panic!("Failed to parse duration string '{duration_str}': unknown unit"),
    };

    let value: u64 = duration_str
        .parse()
        .expect("Failed to parse duration string '{duration_str}': can not parse as u64");

    Ok(Duration::from_secs(value * multiplier))
}

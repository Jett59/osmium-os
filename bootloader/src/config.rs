use core::fmt::{Display, Formatter};

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

#[derive(Debug, Default)]
pub struct Config {
    pub timeout: u32,
    pub default_entry: String,
    pub entries: Vec<ConfigEntry>,
}

#[derive(Debug, Default)]
pub struct ConfigEntry {
    pub label: String,
    pub kernel_path: String,
}

impl ConfigEntry {
    pub fn new(label: String) -> Self {
        ConfigEntry {
            label,
            kernel_path: String::new(),
        }
    }
}

#[derive(Debug)]
pub enum ParseConfigError {
    UnknownKey(String),
}

impl Display for ParseConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseConfigError::UnknownKey(key) => write!(f, "Unknown key: {}", key),
        }
    }
}

pub fn parse_config(config_string: &str) -> Result<Config, ParseConfigError> {
    let mut config: Config = Default::default();
    let mut current_entry = None;

    for line in config_string
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
    {
        if line.starts_with("[") && line.ends_with("]") {
            if let Some(previous_entry) = current_entry {
                config.entries.push(previous_entry);
            }
            let label = line[1..line.len() - 1].trim().to_string();
            current_entry = Some(ConfigEntry::new(label));
        } else if let Some(ref mut entry) = current_entry {
            let mut parts = line.splitn(2, '=');
            let key = parts.next().unwrap().trim();
            let value = parts.next().unwrap().trim();

            match key {
                "kernel" => entry.kernel_path = value.trim_matches('"').to_string(),
                _ => return Err(ParseConfigError::UnknownKey(key.to_string())),
            }
        } else {
            let mut parts = line.splitn(2, '=');
            let key = parts.next().unwrap().trim();
            let value = parts.next().unwrap().trim();

            match key {
                "timeout" => config.timeout = value.parse().unwrap(),
                "default" => config.default_entry = value.to_string(),
                _ => return Err(ParseConfigError::UnknownKey(key.to_string())),
            }
        }
    }
    if let Some(previous_entry) = current_entry {
        config.entries.push(previous_entry);
    }

    Ok(config)
}

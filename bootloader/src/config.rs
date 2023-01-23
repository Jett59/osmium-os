use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use crate::toml::parse_toml;

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

pub fn parse_config(config_string: &str) -> Config {
    let mut config: Config = Default::default();
    
    let toml = parse_toml(config_string);

    //get booot entry from toml
    let boot_entry = toml.get("boot").unwrap();
    config.timeout = boot_entry.get("timeout").unwrap().parse::<u32>().unwrap();
    config.default_entry = boot_entry.get("default").unwrap().clone();

    //iterate over entries excluding boot
    for (key, value) in toml.iter().filter(|(key, _)| key != &"boot") {
        let mut entry = ConfigEntry::new(key.to_string());
        entry.kernel_path = value.get("kernel").unwrap().to_string();
        config.entries.push(entry);
    }

    config
}

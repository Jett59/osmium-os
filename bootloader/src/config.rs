use alloc::string::{String, ToString};
use uefi::Error;

pub struct Config {
    pub kernel: KernelConfig,
}

pub struct KernelConfig {
    pub version: String,
}

pub fn config_from_str(config_string: String) -> Result<Config, Error> {
    //TODO: Figure out how to dynamically create the config struct without serde (which can't compile without std lib)
    //need to work out how to introspect struct properties with macros / traits to do this

    let mut config: Config = Config {
        kernel: KernelConfig {
            version: "0.0.0".to_string()
        }
    };

    let mut section: &str = "";
    config_string.lines().for_each(|line| {
        if line.starts_with("[") && line.ends_with("]") {
            section = line.trim_matches(|c| c == '[' || c == ']');
        } else if section.len() > 0 {
            let key = line.split("=").next().unwrap().trim();
            let value = line.split("=").last().unwrap().trim();
            match section {
                "kernel" => {
                    match key {
                        "version" => {
                            config.kernel.version = value.trim_end_matches(|c| c == '"').trim_start_matches(|c| c == '"').to_string();
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    });
    Ok(config)
}
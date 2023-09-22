use core::str;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;

pub fn parse_toml(file_contents: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut results = BTreeMap::new();

    let lines = file_contents.lines().map(|line| line.trim()).filter(|line| !line.is_empty());

    let mut current_section = "".to_string();
    for line in lines {
        if line.starts_with('[') {
            current_section = line[1..line.len()-1].to_string();
            results.insert(current_section.clone(), BTreeMap::new());
        } else {
            let mut key_value = line.split('=');
            let key = key_value.next().unwrap().trim();
            let value = clean_value(key_value.next().unwrap());
            results.get_mut(&current_section).unwrap().insert(
                key.to_string(),
                value.to_string()
            );
        }
    }

    results
}

fn clean_value(value: &str) -> &str {
    value.trim().trim_matches('"')
}

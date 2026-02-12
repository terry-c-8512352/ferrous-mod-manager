use crate::errors::ModParseError;
use crate::models::ModDescriptor;
use std::fs;
use std::path::Path;

pub fn parse_mod_file(path: &Path) -> Result<ModDescriptor, ModParseError> {
    let contents: String = fs::read_to_string(path)?;
    parse_mod_string(&contents)
}

fn clean_key_value<'a>(key_values: (&'a str, &'a str)) -> (&'a str, &'a str) {
    let (key, value) = key_values;
    let key = key.trim().trim_matches('"');
    let value: &str = value.trim().trim_matches('"');
    (key, value)
}

pub fn parse_mod_string(contents: &str) -> Result<ModDescriptor, ModParseError> {
    let mut name: Option<String> = None;
    let mut path: Option<String> = None;
    let mut supported_version: Option<String> = None;
    let mut remote_file_id: Option<String> = None;
    let mut picture: Option<String> = None;
    let mut version: Option<String> = None;
    let mut tags: Vec<String> = Vec::new();

    let mut in_block = false;

    for line in contents.lines() {
        if line.contains('{') {
            in_block = true;
            continue;
        } else if line.contains('}') {
            in_block = false;
            continue;
        }

        if in_block {
            tags.push(line.trim().trim_matches('"').to_string());
            continue;
        }

        let result = line.split_once("=");

        let (key, value) = match result {
            Some(key_value) => clean_key_value(key_value),
            None => continue,
        };

        match key {
            "name" => name = Some(value.to_string()),
            "path" => path = Some(value.to_string()),
            "supported_version" => supported_version = Some(value.to_string()),
            "remote_file_id" => remote_file_id = Some(value.to_string()),
            "picture" => picture = Some(value.to_string()),
            "version" => version = Some(value.to_string()),
            _ => {}
        }
    }

    let descriptor: ModDescriptor = ModDescriptor {
        tags,
        name: name.ok_or(ModParseError::MissingField("name".into()))?,
        path: path.ok_or(ModParseError::MissingField("path".into()))?,
        supported_version: supported_version
            .ok_or(ModParseError::MissingField("supported_version".into()))?,
        remote_file_id: remote_file_id
            .ok_or(ModParseError::MissingField("remote_file_id".into()))?,
        picture,
        version,
    };

    Ok(descriptor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_descriptor_parser() {
        let test_path = Path::new("tests/fixtures/test_mod.mod");
        let output = parse_mod_file(test_path).unwrap();
        assert_eq!(output.name, "My Mod Name");
    }

    #[test]
    fn test_no_tags() {
        let input =
            "name=\"Test\"\npath=\"/some/path\"\nsupported_version=\"1.0\"\nremote_file_id=\"123\"";
        let output = parse_mod_string(input).unwrap();
        assert_eq!(output.tags.len(), 0);
    }

    #[test]
    fn test_empty_file() {
        let input = "";
        let output = parse_mod_string(input);
        assert!(matches!(output, Err(ModParseError::MissingField(_))))
    }

    #[test]
    #[ignore] // requires local Paradox Interactive Game installation to run
    fn test_real_mod_dir() {
        let mod_dir = std::env::var("GAME_MOD_DIR")
            .expect("Set GAME_MOD_DIR env var to run this test");
        let mod_dir = Path::new(&mod_dir);

        for entry in fs::read_dir(mod_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "mod") {
                let result = parse_mod_file(&path);
                assert!(
                    result.is_ok(),
                    "Failed to parse {:?}: {:?}",
                    path,
                    result.err()
                );
            }
        }

    }
}

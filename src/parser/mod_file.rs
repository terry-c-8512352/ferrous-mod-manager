use crate::models::ModDescriptor;
use std::fs;
use std::io::Error;
use std::path::Path;

pub fn parse_mod_file(path: &Path) -> Result<ModDescriptor, Error> {
    let contents: String = fs::read_to_string(path)?;
    parse_mod_string(&contents)
}

pub fn parse_mod_string(contents: &str) -> Result<ModDescriptor, Error> {

    dbg!(&contents);

    let descriptor: ModDescriptor = ModDescriptor {
        tags: vec!["test".to_string()],
        name: String::from("My Mod Name"),
        path: String::from("example path"),
        supported_version: String::from("1.1.1"),
        remote_file_id: String::from("example_id"),
        picture: None,
        version: None,
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
}
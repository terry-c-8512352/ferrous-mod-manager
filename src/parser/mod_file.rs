use std::io::Error;
use std::fs;
use std::path::Path;
use crate::models::ModDiscriptor;

pub fn parse_mod_file(path: &Path) -> Result<ModDiscriptor, Error> {
    let contents = fs::read_to_string(path)?;
    parse_mod_string(&contents)
}

pub fn parse_mod_string(contents: &str) -> Result<ModDiscriptor, Error> {

    Ok(ModDiscriptor)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_descriptor_parser() {
        let test_path = Path::new("tests/fixtures/test_mod.mod");
        let output = mod_discriptor_parser(test_path).unwrap();
        assert_eq!(output.name, "My Mod Name");
    }

}
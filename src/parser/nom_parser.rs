use nom::branch::alt;
use nom::character::complete::multispace0;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::preceded;
use nom::{
    IResult, Parser, bytes::complete::is_not, character::complete::char, sequence::delimited,
    sequence::separated_pair,
};
use std::fs;
use std::path::Path;
use crate::errors::ModParseError;
use crate::models::ModDescriptor;

#[derive(Debug)]
pub enum ModValue<'a> {
    Single(&'a str),
    List(Vec<&'a str>),
}

// Parses a key value string i.e intro="Hello, World!" -> (intro, Hello, World!)
pub fn parse_key_value(input: &str) -> IResult<&str, (&str, &str)> {
    separated_pair(
        is_not("="),
        char('='),
        delimited(char('"'), is_not("\""), char('"')),
    )
    .parse(input)
}

// Parses a brace delimited tag block i.e tags={ "World", "Other" } -> (tags, [World, Other])
pub fn parse_block_value(input: &str) -> IResult<&str, (&str, Vec<&str>)> {
    separated_pair(
        is_not("="),
        char('='),
        delimited(
            char('{'),
            many0(preceded(
                multispace0,
                delimited(char('"'), is_not("\""), char('"')),
            )),
            preceded(multispace0, char('}')),
        ),
    )
    .parse(input)
}

pub fn parse_mod_file(input: &str) -> Result<ModDescriptor, ModParseError> {
    let mut mod_descriptor = ModDescriptor {
        name: None,
        path: None,
        remote_file_id: None,
        supported_version: None,
        tags: None,
        picture: None,
        version: None
    };

    let parsed_file = many0(preceded(
        multispace0,
        alt((
            map(parse_block_value, |(k, v)| (k, ModValue::List(v))),
            map(parse_key_value, |(k, v)| (k, ModValue::Single(v))),
        )),
    ))
    .parse(input).map_err(|op| ModParseError::ParseError(op.to_string()))?;
    
    for item in &parsed_file.1 {
        match item {
            ("name", ModValue::Single(val)) => mod_descriptor.name = Some(val.to_string()),
            ("path", ModValue::Single(val)) => mod_descriptor.path = Some(val.to_string()),
            ("remote_file_id", ModValue::Single(val)) => mod_descriptor.remote_file_id = Some(val.to_string()),
            ("supported_version", ModValue::Single(val)) => mod_descriptor.supported_version = Some(val.to_string()),
            ("picture", ModValue::Single(val)) => mod_descriptor.picture = Some(val.to_string()),
            ("version", ModValue::Single(val)) => mod_descriptor.version = Some(val.to_string()),
            ("tags", ModValue::List(list)) => mod_descriptor.tags = Some(list.into_iter().map(|f| f.to_string()).collect()),
            _ => {} // TODO: Probs raise an error here? Probably means we have an unexpected field?
            
        }
    }

    // Verify required fields
    mod_descriptor.name.as_ref().ok_or(ModParseError::MissingField(("name".to_string())))?;
    mod_descriptor.path.as_ref().ok_or(ModParseError::MissingField(("path".to_string())))?;
    mod_descriptor.remote_file_id.as_ref().ok_or(ModParseError::MissingField(("remote_file_id".to_string())))?;
    mod_descriptor.supported_version.as_ref().ok_or(ModParseError::MissingField(("supported_version".to_string())))?;


    Ok(mod_descriptor)
}

pub fn read_mod_file_from_disk(path: &Path) {
    let contents: String = fs::read_to_string(path).unwrap();
    let output = parse_mod_file(&contents);
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_value() {
        let input = r#"name="My Mod Name""#;
        let (remaining, (key, value)) = parse_key_value(input).unwrap();
        assert_eq!(key, "name");
        assert_eq!(value, "My Mod Name");
        assert_eq!(remaining, "");
    }

    #[test]
    fn test_block_parser() {
        let input = "tags={ \"Galaxy Generation\" \"Gameplay\" }";
        let parsed_block = parse_block_value(input).unwrap().1;
    }

    #[test]
    fn test_empty_file() {
        let input = "";
        let parsed_file = parse_mod_file(input);
        assert!(matches!(parsed_file, Err(ModParseError::MissingField(_))))
    }

}

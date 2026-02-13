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
use std::process::Output;

#[derive(Debug)]
enum ModValue<'a> {
    Single(&'a str),
    List(Vec<&'a str>),
}

fn parse_key_value(input: &str) -> IResult<&str, (&str, &str)> {
    let output = separated_pair(
        is_not("="),
        char('='),
        delimited(char('"'), is_not("\""), char('"')),
    )
    .parse(input);
    output
}

fn parse_block_value(input: &str) -> IResult<&str, (&str, Vec<&str>)> {
    let output = separated_pair(
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
    .parse(input);
    output
}

fn parse_mod_file(input: &str) -> IResult<&str, Vec<(&str, ModValue)>> {
    let output = many0(preceded(
        multispace0,
        alt((
            map(parse_block_value, |(k, v)| (k, ModValue::List(v))),
            map(parse_key_value, |(k, v)| (k, ModValue::Single(v))),
        )),
    ))
    .parse(input);
    output
}

fn read_mod_file_from_disk(path: &Path) {
    let contents: String = fs::read_to_string(path).unwrap();
    let output = parse_mod_file(&contents);
    dbg!(output);
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
        let parsed_block = parse_block_value(input).unwrap();
        dbg!(parsed_block);
    }

    #[test]
    fn test_mock_mod_file() {
        let test_path = Path::new("tests/fixtures/test_mod.mod");
        let output = read_mod_file_from_disk(test_path);
        dbg!(output);
    }
}

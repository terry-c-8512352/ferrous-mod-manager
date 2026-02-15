use crate::errors::VdfParseError;
use crate::models::LibraryVdf;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::take_while,
    character::complete::{char, multispace0, multispace1},
    combinator::map,
    multi::many0,
    sequence::{delimited, preceded, separated_pair},
};

#[derive(Debug)]
pub enum VdfEntry<'a> {
    KeyValue(&'a str, &'a str),
    Block(&'a str, Vec<(&'a str, &'a str)>),
}

pub fn parse_quoted_string(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), take_while(|c| c != '"'), char('"')).parse(input)
}

pub fn parse_tabbed_kv_pair(input: &str) -> IResult<&str, (&str, &str)> {
    separated_pair(parse_quoted_string, multispace1, parse_quoted_string).parse(input)
}

pub fn parse_apps_block(input: &str) -> IResult<&str, (&str, Vec<(&str, &str)>)> {
    separated_pair(
        parse_quoted_string,
        multispace1,
        delimited(
            char('{'),
            many0(preceded(multispace0, parse_tabbed_kv_pair)),
            preceded(multispace0, char('}')),
        ),
    )
    .parse(input)
}

pub fn parse_vdf_block(input: &str) -> Result<LibraryVdf, VdfParseError> {
    let (_, (vdf_idx, entries)) = separated_pair(
        parse_quoted_string,
        multispace1,
        delimited(
            char('{'),
            many0(preceded(
                multispace0,
                alt((
                    map(parse_apps_block, |(k, v)| VdfEntry::Block(k, v)),
                    map(parse_tabbed_kv_pair, |(k, v)| VdfEntry::KeyValue(k, v)),
                )),
            )),
            preceded(multispace0, char('}')),
        ),
    )
    .parse(input)
    .map_err(|e| VdfParseError::ParseError(e.to_string()))?;

    let mut path: Option<String> = None;
    let mut apps: Vec<u32> = vec![];
    let idx = vdf_idx.parse::<u32>()?; // TODO: Error better

    for entry in entries {
        match entry {
            VdfEntry::KeyValue("path", value) => {
                path = Some(value.to_string());
            }
            VdfEntry::Block("apps", app_list) => {
                for (app_id, _size) in app_list { // Size of games is not currently used
                    apps.push(app_id.parse().unwrap());
                }
            }
            _ => {}
        }
    }

    let path = path.ok_or(VdfParseError::MissingField("path".to_string()))?;

    let lib_vdf = LibraryVdf { idx, path, apps };
    Ok(lib_vdf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_quoted_string() {
        let input = "\"test\"";
        let (remaining, result) = parse_quoted_string(input).unwrap();
        assert_eq!(remaining, "");
        assert_eq!(result, "test");
    }

    #[test]
    fn test_tabbed_key_value_pair() {
        let input = r#""test_key"            "test_value""#;
        let (remaining, result) = parse_tabbed_kv_pair(input).unwrap();
        assert_eq!(remaining, "");
        assert_eq!(result.0, "test_key");
        assert_eq!(result.1, "test_value")
    }

    #[test]
    fn test_apps_block() {
        let input = r#""apps"
		{
			"123"		"1234567345"
			"12345"		"12345454534"
		}"#;

        let (remaining, result) = parse_apps_block(input).unwrap();
        assert_eq!(remaining, "");
        assert_eq!(result.0, "apps");
        assert_eq!(result.1.len(), 2);
        assert_eq!(result.1[0], ("123", "1234567345"));
        assert_eq!(result.1[1], ("12345", "12345454534"));
    }

    #[test]
    fn test_vdf_block() {
        let input = r#""0"
        {
            "path"		"/home/user/.local/share/Steam"
            "label"		""
            "contentid"		"26525186198543"
            "apps"
            {
                "123"		"1234567345"
			    "12345"		"12345454534"
            }
        }"#;

        let lib_vdf = parse_vdf_block(input).unwrap();
        assert_eq!(lib_vdf.apps[0], 123);
        assert_eq!(lib_vdf.apps[1], 12345);
        assert_eq!(lib_vdf.idx, 0);
        assert_eq!(lib_vdf.path, "/home/user/.local/share/Steam");
    }
}

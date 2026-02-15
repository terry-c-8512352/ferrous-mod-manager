use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::is_not,
    character::complete::{char, multispace0, multispace1},
    multi::many0,
    sequence::{delimited, preceded, separated_pair},
};

pub fn parse_quoted_string(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), is_not("\""), char('"')).parse(input)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

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
        dbg!(remaining);
        dbg!(result);
    }

    #[test]
    fn test_vdf_block() {
        let input = r#""0"
        {
            "path"		"/home/user/.local/share/Steam"
            "label"		""
            "contentid"		"26525186198543"
            ...
            "apps"
            {
                "123"		"1234567345"
			    "12345"		"12345454534"
            }
        }"#;
    }

    #[test]
    fn test_parse_vdf() {
        let contents =
            fs::read_to_string("/home/tyrcho/.steam/steam/steamapps/libraryfolders.vdf").unwrap();
        dbg!(contents);
    }
}

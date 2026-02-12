use nom::{
    IResult,
    Parser,
    sequence::delimited,
    character::complete::char,
    bytes::complete::is_not,
};

fn parse_quoted_string(input: &str) -> IResult<&str, &str> {
    delimited(char('"'), is_not("\""), char('"')).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_quoted_string() {
        let input = "\"Hello, World!\"";
        let output = parse_quoted_string(input).unwrap();
        assert_eq!(output.1, "Hello, World!");
        dbg!(output);
    }
}
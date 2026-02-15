use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModParseError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Mod descriptor parsing failed: {0}")]
    ParseError(String),
}

#[derive(Error, Debug)]
pub enum VdfParseError {
    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("VDF parsing failed: {0}")]
    ParseError(String),

    #[error("Invalid number: {0}")]
    InvalidNumber(#[from] std::num::ParseIntError),
}

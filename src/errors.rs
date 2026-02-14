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

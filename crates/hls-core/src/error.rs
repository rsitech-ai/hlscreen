use thiserror::Error;

pub type HlsResult<T> = Result<T, HlsError>;

#[derive(Debug, Error)]
pub enum HlsError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("symbol error: {0}")]
    Symbol(String),

    #[error("time error: {0}")]
    Time(String),

    #[error("TOML decode error: {0}")]
    TomlDecode(#[from] toml::de::Error),

    #[error("TOML encode error: {0}")]
    TomlEncode(#[from] toml::ser::Error),

    #[error("external service error: {0}")]
    External(String),
}

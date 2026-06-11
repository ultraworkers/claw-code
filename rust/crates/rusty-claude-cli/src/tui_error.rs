use std::fmt;

#[derive(Debug)]
pub enum TuiError {
    Io(std::io::Error),
    Terminal(String),
    Runtime(String),
    Config(String),
}

impl fmt::Display for TuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TuiError::Io(e) => write!(f, "TUI I/O error: {e}"),
            TuiError::Terminal(msg) => write!(f, "Terminal error: {msg}"),
            TuiError::Runtime(msg) => write!(f, "Runtime error: {msg}"),
            TuiError::Config(msg) => write!(f, "Config error: {msg}"),
        }
    }
}

impl std::error::Error for TuiError {}

impl From<std::io::Error> for TuiError {
    fn from(e: std::io::Error) -> Self {
        TuiError::Io(e)
    }
}

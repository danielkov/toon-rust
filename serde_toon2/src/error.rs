use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    InvalidSyntax,
    InvalidEscape,
    UnterminatedString,
    MissingColon,
    IndentationError,
    BlankLineInArray,
    CountMismatch,
    WidthMismatch,
    ExpansionConflict,
    DelimiterMismatch,
    InvalidHeader,
    Io(String),
    Custom(String),
}

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Error {
            kind,
            message: message.into(),
            line: None,
            column: None,
        }
    }

    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    pub fn custom(msg: impl Into<String>) -> Self {
        let message = msg.into();
        Error::new(ErrorKind::Custom(message.clone()), message)
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let (Some(line), Some(col)) = (self.line, self.column) {
            write!(f, "{} at line {}, column {}", self.message, line, col)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::custom(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::custom(msg.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::new(ErrorKind::Io(err.to_string()), err.to_string())
    }
}

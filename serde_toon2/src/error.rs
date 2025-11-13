//! Error types for TOON serialization and deserialization.
//!
//! This module provides strongly-typed error handling with location information
//! to aid in debugging TOON format issues.

use std::fmt;

/// A specialized `Result` type for TOON operations.
pub type Result<T> = std::result::Result<T, Error>;

/// The main error type for TOON operations.
///
/// Errors include a specific kind, a descriptive message, and optional line/column
/// location information for parse errors.
///
/// # Examples
///
/// ```
/// use serde_toon2::{from_str, Error};
/// use serde_json::Value;
///
/// let invalid = "key: [3]: a,b";  // count mismatch
/// let result: Result<Value, Error> = from_str(invalid);
///
/// if let Err(err) = result {
///     println!("Error: {}", err);
///     // Error messages include location when available
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

/// Specific kinds of errors that can occur during TOON operations.
///
/// Each variant represents a distinct error condition with semantic meaning.
#[derive(Debug, Clone)]
pub enum ErrorKind {
    /// Invalid TOON syntax was encountered.
    InvalidSyntax,
    /// An invalid escape sequence was found in a string.
    InvalidEscape,
    /// A string was opened but not properly closed.
    UnterminatedString,
    /// A key-value pair is missing the required colon separator.
    MissingColon,
    /// Indentation doesn't match the expected level or is inconsistent.
    IndentationError,
    /// A blank line was found within an array (not allowed).
    BlankLineInArray,
    /// The number of array elements doesn't match the declared count.
    ///
    /// # Example
    /// ```text
    /// items[3]: a,b  // CountMismatch: declared 3, found 2
    /// ```
    CountMismatch,
    /// The number of fields in an array row doesn't match the header.
    ///
    /// # Example
    /// ```text
    /// users[2 name,age]:
    ///   Alice,30
    ///   Bob          // WidthMismatch: expected 2 fields, found 1
    /// ```
    WidthMismatch,
    /// Path expansion resulted in conflicting values.
    ExpansionConflict,
    /// Array delimiter doesn't match the declared delimiter.
    DelimiterMismatch,
    /// Array header syntax is invalid.
    InvalidHeader,
    /// An I/O error occurred during reading or writing.
    Io(String),
    /// A custom error message.
    Custom(String),
}

impl Error {
    /// Creates a new error with the specified kind and message.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::error::{Error, ErrorKind};
    ///
    /// let err = Error::new(ErrorKind::InvalidSyntax, "unexpected character");
    /// // Can access the kind
    /// let _ = err.kind();
    /// ```
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Error {
            kind,
            message: message.into(),
            line: None,
            column: None,
        }
    }

    /// Adds location information to this error.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::error::{Error, ErrorKind};
    ///
    /// let err = Error::new(ErrorKind::MissingColon, "expected ':'")
    ///     .with_location(5, 12);
    ///
    /// let msg = format!("{}", err);
    /// assert!(msg.contains("line 5"));
    /// assert!(msg.contains("column 12"));
    /// ```
    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    /// Creates a custom error with a free-form message.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::error::Error;
    ///
    /// let err = Error::custom("something went wrong");
    /// println!("{}", err);
    /// ```
    pub fn custom(msg: impl Into<String>) -> Self {
        let message = msg.into();
        Error::new(ErrorKind::Custom(message.clone()), message)
    }

    /// Returns the kind of this error.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::error::{Error, ErrorKind};
    ///
    /// let err = Error::new(ErrorKind::CountMismatch, "array length mismatch");
    /// match err.kind() {
    ///     ErrorKind::CountMismatch => println!("Count mismatch detected"),
    ///     _ => {}
    /// }
    /// ```
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

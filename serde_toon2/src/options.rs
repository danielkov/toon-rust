//! Configuration options for TOON serialization and deserialization.

use std::fmt;

/// Options for controlling TOON serialization behavior.
///
/// # Examples
///
/// ```
/// use serde_toon2::{EncoderOptions, Delimiter, KeyFolding};
///
/// let opts = EncoderOptions {
///     indent: 4,
///     delimiter: Delimiter::Pipe,
///     key_folding: KeyFolding::Off,
///     flatten_depth: usize::MAX,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct EncoderOptions {
    /// Number of spaces per indentation level.
    ///
    /// Default: `2`
    pub indent: usize,

    /// Delimiter to use for array elements.
    ///
    /// Default: [`Delimiter::Comma`]
    pub delimiter: Delimiter,

    /// Whether to compress nested object paths.
    ///
    /// Default: [`KeyFolding::Off`]
    pub key_folding: KeyFolding,

    /// Maximum depth at which to inline nested structures.
    ///
    /// Default: `usize::MAX` (inline everything possible)
    pub flatten_depth: usize,
}

impl Default for EncoderOptions {
    fn default() -> Self {
        EncoderOptions {
            indent: 2,
            delimiter: Delimiter::Comma,
            key_folding: KeyFolding::Off,
            flatten_depth: usize::MAX,
        }
    }
}

/// Options for controlling TOON deserialization behavior.
///
/// # Examples
///
/// ```
/// use serde_toon2::{DecoderOptions, PathExpansion};
///
/// // Enable strict validation
/// let opts = DecoderOptions {
///     indent: 2,
///     strict: true,
///     expand_paths: PathExpansion::Off,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct DecoderOptions {
    /// Expected number of spaces per indentation level.
    ///
    /// Default: `2`
    pub indent: usize,

    /// Enable strict validation mode.
    ///
    /// When enabled, enforces additional constraints defined in the TOON specification.
    ///
    /// Default: `false`
    pub strict: bool,

    /// Whether to expand dot-notation paths into nested objects.
    ///
    /// Default: [`PathExpansion::Off`]
    pub expand_paths: PathExpansion,
}

impl Default for DecoderOptions {
    fn default() -> Self {
        DecoderOptions {
            indent: 2,
            strict: false,
            expand_paths: PathExpansion::Off,
        }
    }
}

/// Delimiter characters used to separate array elements.
///
/// # Examples
///
/// ```
/// use serde_toon2::{to_string_with_options, EncoderOptions, Delimiter};
///
/// let data = vec!["a", "b", "c"];
///
/// // Comma-delimited (default)
/// let opts = EncoderOptions {
///     delimiter: Delimiter::Comma,
///     ..Default::default()
/// };
/// let toon = to_string_with_options(&data, opts).unwrap();
/// // Output: "[3]: a,b,c"
///
/// // Tab-delimited
/// let opts = EncoderOptions {
///     delimiter: Delimiter::Tab,
///     ..Default::default()
/// };
/// let toon = to_string_with_options(&data, opts).unwrap();
/// // Output: "[3]\t: a\tb\tc"
///
/// // Pipe-delimited
/// let opts = EncoderOptions {
///     delimiter: Delimiter::Pipe,
///     ..Default::default()
/// };
/// let toon = to_string_with_options(&data, opts).unwrap();
/// // Output: "[3]|: a|b|c"
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delimiter {
    /// Comma delimiter: `items[3]: a,b,c`
    Comma,
    /// Tab delimiter: `items[3]\t: a\tb\tc`
    Tab,
    /// Pipe delimiter: `items[3]|: a|b|c`
    Pipe,
}

impl Delimiter {
    /// Returns the delimiter as a character.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Delimiter;
    ///
    /// assert_eq!(Delimiter::Comma.as_char(), ',');
    /// assert_eq!(Delimiter::Tab.as_char(), '\t');
    /// assert_eq!(Delimiter::Pipe.as_char(), '|');
    /// ```
    pub fn as_char(&self) -> char {
        match self {
            Delimiter::Comma => ',',
            Delimiter::Tab => '\t',
            Delimiter::Pipe => '|',
        }
    }

    /// Returns the delimiter as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Delimiter;
    ///
    /// assert_eq!(Delimiter::Comma.as_str(), ",");
    /// assert_eq!(Delimiter::Tab.as_str(), "\t");
    /// assert_eq!(Delimiter::Pipe.as_str(), "|");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Delimiter::Comma => ",",
            Delimiter::Tab => "\t",
            Delimiter::Pipe => "|",
        }
    }

    /// Returns the marker that appears in array headers.
    ///
    /// Comma uses no marker, while tab and pipe use their respective characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_toon2::Delimiter;
    ///
    /// assert_eq!(Delimiter::Comma.header_marker(), "");
    /// assert_eq!(Delimiter::Tab.header_marker(), "\t");
    /// assert_eq!(Delimiter::Pipe.header_marker(), "|");
    /// ```
    pub fn header_marker(&self) -> &'static str {
        match self {
            Delimiter::Comma => "",
            Delimiter::Tab => "\t",
            Delimiter::Pipe => "|",
        }
    }
}

impl fmt::Display for Delimiter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Delimiter::Comma => write!(f, "comma"),
            Delimiter::Tab => write!(f, "tab"),
            Delimiter::Pipe => write!(f, "pipe"),
        }
    }
}

/// Controls whether nested object keys are compressed during serialization.
///
/// # Examples
///
/// ```text
/// // KeyFolding::Off (default)
/// user:
///   profile:
///     name: Ada
///
/// // KeyFolding::Safe
/// user.profile.name: Ada
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFolding {
    /// No path compression (default).
    Off,
    /// Compress nested object paths when safe.
    Safe,
}

/// Controls whether dot-notation paths are expanded during deserialization.
///
/// # Examples
///
/// ```text
/// // With PathExpansion::Safe, this:
/// user.name: Ada
///
/// // Becomes:
/// user:
///   name: Ada
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathExpansion {
    /// No path expansion (default).
    Off,
    /// Expand dot-notation paths when safe.
    Safe,
}

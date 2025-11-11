use std::fmt;

#[derive(Debug, Clone)]
pub struct EncoderOptions {
    pub indent: usize,
    pub delimiter: Delimiter,
    pub key_folding: KeyFolding,
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

#[derive(Debug, Clone)]
pub struct DecoderOptions {
    pub indent: usize,
    pub strict: bool,
    pub expand_paths: PathExpansion,
}

impl Default for DecoderOptions {
    fn default() -> Self {
        DecoderOptions {
            indent: 2,
            strict: true,
            expand_paths: PathExpansion::Off,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delimiter {
    Comma,
    Tab,
    Pipe,
}

impl Delimiter {
    pub fn as_char(&self) -> char {
        match self {
            Delimiter::Comma => ',',
            Delimiter::Tab => '\t',
            Delimiter::Pipe => '|',
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Delimiter::Comma => ",",
            Delimiter::Tab => "\t",
            Delimiter::Pipe => "|",
        }
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyFolding {
    Off,
    Safe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathExpansion {
    Off,
    Safe,
}

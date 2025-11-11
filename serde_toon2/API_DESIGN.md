# serde_toon2 API Design

## Overview

`serde_toon2` provides a serde-compatible TOON (Token-Oriented Object Notation) serialization library that mirrors the `serde_json` API as closely as possible.

## Module Structure

```
serde_toon2/
├── lib.rs           # Public API exports
├── value.rs         # Value enum (mirrors serde_json::Value)
├── ser.rs           # Serialization implementation
├── de.rs            # Deserialization implementation
├── error.rs         # Error types
├── options.rs       # Configuration options
└── tests/           # Test fixtures
```

## Public API

### Top-Level Functions

#### Deserialization
```rust
// Deserialize from TOON string
pub fn from_str<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T>

// Deserialize from TOON bytes
pub fn from_slice<'a, T: Deserialize<'a>>(v: &'a [u8]) -> Result<T>

// Deserialize from reader
pub fn from_reader<R: Read, T: DeserializeOwned>(rdr: R) -> Result<T>

// Deserialize with options
pub fn from_str_with_options<'a, T: Deserialize<'a>>(
    s: &'a str,
    options: DecoderOptions
) -> Result<T>
```

#### Serialization
```rust
// Serialize to TOON string
pub fn to_string<T: Serialize>(value: &T) -> Result<String>

// Serialize to TOON bytes
pub fn to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>>

// Serialize to writer
pub fn to_writer<W: Write, T: Serialize>(writer: W, value: &T) -> Result<()>

// Serialize with options
pub fn to_string_with_options<T: Serialize>(
    value: &T,
    options: EncoderOptions
) -> Result<String>
```

### Configuration Types

```rust
#[derive(Debug, Clone)]
pub struct EncoderOptions {
    pub indent: usize,              // Default: 2
    pub delimiter: Delimiter,        // Default: Comma
    pub key_folding: KeyFolding,     // Default: Off
    pub flatten_depth: usize,        // Default: usize::MAX
}

#[derive(Debug, Clone)]
pub struct DecoderOptions {
    pub indent: usize,              // Default: 2
    pub strict: bool,               // Default: true
    pub expand_paths: PathExpansion, // Default: Off
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delimiter {
    Comma,
    Tab,
    Pipe,
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
```

### Value Type

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(Map<String, Value>),
}

impl Value {
    pub fn as_bool(&self) -> Option<bool>
    pub fn as_i64(&self) -> Option<i64>
    pub fn as_f64(&self) -> Option<f64>
    pub fn as_str(&self) -> Option<&str>
    pub fn as_array(&self) -> Option<&Vec<Value>>
    pub fn as_object(&self) -> Option<&Map<String, Value>>
    // ... etc
}
```

### Error Type

```rust
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

#[derive(Debug)]
pub enum ErrorKind {
    // Syntax errors
    InvalidSyntax,
    InvalidEscape,
    UnterminatedString,
    MissingColon,

    // Structural errors
    IndentationError,
    BlankLineInArray,

    // Count/width errors
    CountMismatch,
    WidthMismatch,

    // Path expansion errors
    ExpansionConflict,

    // Serialization errors
    Custom(String),
}
```

## Implementation Strategy

### Phase 1: Value & Error Types
- Implement `Value` enum with helper methods
- Implement `Error` type with Display trait
- Basic configuration structs

### Phase 2: Serialization
- Implement `Serializer` that converts `T: Serialize` to TOON format
- Handle all TOON data types (primitives, objects, arrays, tabular)
- Support delimiter variations
- Implement key folding (optional)

### Phase 3: Deserialization
- Implement tokenizer/lexer for TOON format
- Implement `Deserializer` that converts TOON to `T: Deserialize`
- Handle all TOON forms (root detection, objects, arrays, etc.)
- Implement strict mode validation
- Implement path expansion (optional)

### Phase 4: Testing
- Build comprehensive test fixtures
- Ensure all spec fixtures pass
- Test reversibility (JSON ↔ TOON ↔ JSON)

# serde_toon2

Serde-compatible serializer/deserializer for TOON (Token-Oriented Object Notation), a line-oriented, indentation-based format that encodes the JSON data model.

## Installation

```toml
[dependencies]
serde_toon2 = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
```

## Format Overview

TOON encodes JSON structures using indentation instead of braces:

```
user:
  id: 123
  name: Ada
items[2]: a,b
```

Equivalent JSON:

```json
{ "user": { "id": 123, "name": "Ada" }, "items": ["a", "b"] }
```

Key characteristics:

- **Indentation-based structure**: Objects use 2-space indentation (configurable)
- **Minimal quoting**: Strings quoted only when ambiguous (reserved words, delimiters, special syntax)
- **Array headers**: Declare length and optional field list: `[3]` or `[3 name,age]`
- **Three delimiters**: Arrays use comma (default), tab, or pipe delimiters

## Serialization

```rust
use serde::Serialize;
use serde_toon2::{to_string, to_string_with_options, EncoderOptions, Delimiter};

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
}

let user = User { id: 42, name: "Ada".to_string() };

// Default options
let toon = to_string(&user)?;
// Output: "id: 42\nname: Ada"

// Custom options
let opts = EncoderOptions {
    indent: 4,
    delimiter: Delimiter::Pipe,
    ..Default::default()
};
let toon = to_string_with_options(&user, &opts)?;
```

### Encoder Options

```rust
pub struct EncoderOptions {
    pub indent: usize,           // Spaces per indent level (default: 2)
    pub delimiter: Delimiter,    // Array delimiter (default: Comma)
    pub key_folding: KeyFolding, // Path compression (default: Off)
    pub flatten_depth: usize,    // Max depth to inline (default: MAX)
}

pub enum Delimiter {
    Comma,  // items[3]: a,b,c
    Tab,    // items[3]\t: a\tb\tc
    Pipe,   // items[3]|: a|b|c
}
```

## Deserialization

```rust
use serde::Deserialize;
use serde_toon2::{from_str, from_str_with_options, DecoderOptions};

#[derive(Deserialize)]
struct User {
    id: u64,
    name: String,
}

let toon = "id: 42\nname: Ada";
let user: User = from_str(toon)?;

// Strict mode validation
let opts = DecoderOptions {
    strict: true,
    ..Default::default()
};
let user: User = from_str_with_options(toon, &opts)?;
```

### Decoder Options

```rust
pub struct DecoderOptions {
    pub indent: usize,                   // Expected indent size (default: 2)
    pub strict: bool,                    // Enable strict validation (default: false)
    pub expand_paths: PathExpansion,     // Path notation handling (default: Off)
}
```

## API

### Serialization

- `to_string<T: Serialize>(value: &T) -> Result<String>`
- `to_string_with_options<T: Serialize>(value: &T, options: &EncoderOptions) -> Result<String>`
- `to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>>`
- `to_vec_with_options<T: Serialize>(value: &T, options: &EncoderOptions) -> Result<Vec<u8>>`
- `to_writer<W: Write, T: Serialize>(writer: W, value: &T) -> Result<()>`
- `to_writer_with_options<W: Write, T: Serialize>(writer: W, value: &T, options: &EncoderOptions) -> Result<()>`

### Deserialization

- `from_str<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T>`
- `from_str_with_options<'a, T: Deserialize<'a>>(s: &'a str, options: &DecoderOptions) -> Result<T>`
- `from_slice<'a, T: Deserialize<'a>>(v: &'a [u8]) -> Result<T>`
- `from_slice_with_options<'a, T: Deserialize<'a>>(v: &'a [u8], options: &DecoderOptions) -> Result<T>`
- `from_reader<R: Read, T: DeserializeOwned>(reader: R) -> Result<T>`
- `from_reader_with_options<R: Read, T: DeserializeOwned>(reader: R, options: &DecoderOptions) -> Result<T>`

## Error Handling

Strongly-typed errors with location information:

```rust
pub enum ErrorKind {
    InvalidSyntax,
    InvalidEscape,
    UnterminatedString,
    MissingColon,
    IndentationError,
    BlankLineInArray,
    CountMismatch,      // Array length mismatch
    WidthMismatch,      // Field count mismatch
    DelimiterMismatch,
    InvalidHeader,
    // ...
}
```

Errors include line/column location when available:

```
Invalid syntax at line 5, column 12
```

## Value Type

Generic value type for dynamic content:

```rust
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(Map<String, Value>),
}

pub type Map<K, V> = indexmap::IndexMap<K, V>; // Preserves insertion order
```

## Format Examples

### Objects

```
name: Ada
age: 42
active: true
```

### Nested Objects

```
user:
  name: Ada
  profile:
    bio: Programmer
    location: London
```

### Arrays (comma-delimited)

```
tags[3]: rust,serde,parser
```

### Arrays (vertical)

```
tags[3]:
  rust
  serde
  parser
```

### Arrays of Objects

```
users[2]:
  name: Ada
  age: 42
  ---
  name: Bob
  age: 35
```

### Arrays with Field Lists

```
users[2 name,age]:
  Ada,42
  Bob,35
```

### Tab-Delimited Arrays

```
scores[3]	: 95	87	92
```

### Pipe-Delimited Arrays

```
paths[2]|: /usr/bin|/usr/local/bin
```

## Dependencies

- `serde` 1.0 - Serialization framework
- `indexmap` 2.0 - Order-preserving maps
- `regex` 1.0 - Pattern matching

## License

MIT

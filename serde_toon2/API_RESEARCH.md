# API Research: serde and serde_json

## Core Serde Patterns

### Traits

- `Serialize` - Types that can be serialized
- `Deserialize` - Types that can be deserialized
- Both can be auto-derived using `#[derive(Serialize, Deserialize)]`

### serde_json API Structure

#### Deserialization Functions

```rust
pub fn from_str<'a, T>(s: &'a str) -> Result<T> where T: Deserialize<'a>
pub fn from_slice<'a, T>(v: &'a [u8]) -> Result<T> where T: Deserialize<'a>
pub fn from_reader<R, T>(rdr: R) -> Result<T> where R: Read, T: DeserializeOwned
pub fn from_value<T>(value: Value) -> Result<T> where T: DeserializeOwned
```

#### Serialization Functions

```rust
pub fn to_string<T: ?Sized>(value: &T) -> Result<String> where T: Serialize
pub fn to_string_pretty<T: ?Sized>(value: &T) -> Result<String> where T: Serialize
pub fn to_vec<T: ?Sized>(value: &T) -> Result<Vec<u8>> where T: Serialize
pub fn to_writer<W, T: ?Sized>(writer: W, value: &T) -> Result<()> where W: Write, T: Serialize
pub fn to_value<T>(value: T) -> Result<Value> where T: Serialize
```

#### Value Type

- `serde_json::Value` - Untyped JSON value enum
- Variants: Null, Bool, Number, String, Array, Object

## Design Principles for serde_toon2

1. **API Compatibility**: Mirror serde_json API as closely as possible
2. **Configuration**: Support TOON-specific options (indent, delimiter, strict mode, etc.)
3. **Error Handling**: Use Result types with descriptive errors
4. **Reversibility**: Ensure JSON ↔ TOON ↔ JSON and TOON ↔ JSON ↔ TOON work correctly

## Proposed API for serde_toon2

### Deserialization

```rust
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
pub fn from_slice<'a, T>(v: &'a [u8]) -> Result<T>
pub fn from_reader<R, T>(rdr: R) -> Result<T>
```

### Serialization

```rust
pub fn to_string<T>(value: &T) -> Result<String>
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
```

### Configuration

- Support encoder options: indent, delimiter, keyFolding, flattenDepth
- Support decoder options: indent, strict, expandPaths

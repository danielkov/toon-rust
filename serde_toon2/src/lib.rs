//! Serde-compatible serializer and deserializer for TOON (Token-Oriented Object Notation).
//!
//! TOON is a line-oriented, indentation-based format that encodes the JSON data model.
//! It emphasizes human readability with minimal punctuation while maintaining compatibility
//! with the JSON type system.
//!
//! # Features
//!
//! - **Indentation-based structure**: Objects use indentation instead of braces
//! - **Minimal quoting**: Strings quoted only when necessary
//! - **Array headers**: Declare length and optional field lists
//! - **Multiple delimiters**: Arrays can use comma, tab, or pipe delimiters
//! - **Serde integration**: Full compatibility with Rust's serde ecosystem
//!
//! # Usage
//!
//! ## Serialization
//!
//! ```
//! use serde::Serialize;
//! use serde_toon2::to_string;
//!
//! #[derive(Serialize)]
//! struct User {
//!     id: u64,
//!     name: String,
//! }
//!
//! let user = User {
//!     id: 42,
//!     name: "Ada".to_string()
//! };
//!
//! let toon = to_string(&user).unwrap();
//! assert_eq!(toon, "id: 42\nname: Ada");
//! ```
//!
//! ## Deserialization
//!
//! ```
//! use serde::Deserialize;
//! use serde_toon2::from_str;
//!
//! #[derive(Deserialize, Debug, PartialEq)]
//! struct User {
//!     id: u64,
//!     name: String,
//! }
//!
//! let toon = "id: 42\nname: Ada";
//! let user: User = from_str(toon).unwrap();
//!
//! assert_eq!(user, User { id: 42, name: "Ada".to_string() });
//! ```
//!
//! # Format Examples
//!
//! ## Simple Object
//!
//! ```text
//! name: Ada
//! age: 42
//! active: true
//! ```
//!
//! ## Nested Objects
//!
//! ```text
//! user:
//!   name: Ada
//!   profile:
//!     bio: Programmer
//!     location: London
//! ```
//!
//! ## Arrays
//!
//! Inline with commas:
//! ```text
//! tags[3]: rust,serde,parser
//! ```
//!
//! Vertical:
//! ```text
//! tags[3]:
//!   rust
//!   serde
//!   parser
//! ```
//!
//! ## Arrays with Field Lists
//!
//! ```text
//! users[2 name,age]:
//!   Ada,42
//!   Bob,35
//! ```
//!
//! # Configuration Options
//!
//! Both serialization and deserialization can be customized using options:
//!
//! ```
//! use serde_toon2::{to_string_with_options, EncoderOptions, Delimiter};
//!
//! let opts = EncoderOptions {
//!     indent: 4,
//!     delimiter: Delimiter::Pipe,
//!     ..Default::default()
//! };
//!
//! let data = vec!["a", "b", "c"];
//! let toon = to_string_with_options(&data, opts).unwrap();
//! ```
//!
//! # Error Handling
//!
//! Errors include location information when available:
//!
//! ```should_panic
//! use serde_toon2::from_str;
//! use serde_json::Value;
//!
//! let invalid = "key: unclosed quote\"";
//! let result: Result<Value, _> = from_str(invalid);
//!
//! // Error includes line and column information
//! assert!(result.is_err());
//! ```

pub mod de;
pub mod error;
pub mod options;
pub mod ser;
pub mod value;

pub use de::{
    from_reader, from_reader_with_options, from_slice, from_slice_with_options, from_str,
    from_str_with_options,
};
pub use error::{Error, Result};
pub use options::{DecoderOptions, Delimiter, EncoderOptions, KeyFolding, PathExpansion};
pub use ser::{
    to_string, to_string_with_options, to_vec, to_vec_with_options, to_writer,
    to_writer_with_options,
};
pub use value::{Map, Number, Value};

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust workspace implementing the TOON (Token-Oriented Object Notation) format - a line-oriented, indentation-based text format that encodes the JSON data model. The workspace contains two crates:

- **serde_toon2**: A serde-compatible serializer and deserializer for TOON format
- **cli**: A command-line tool that converts between JSON and TOON formats

## Build and Test Commands

```bash
# Build entire workspace
cargo build

# Build specific crate
cargo build -p serde_toon2
cargo build -p toon

# Run all tests
cargo test

# Run tests for specific crate
cargo test -p serde_toon2
cargo test -p toon

# Run a single test
cargo test -p serde_toon2 --test test_fixtures -- encode_fixtures
cargo test -p serde_toon2 test_name

# Run the CLI
cargo run -p toon -- <input>

# Examples:
cargo run -p toon -- https://api.github.com/users
cargo run -p toon -- data.json
cargo run -p toon -- '{"name":"Alice","age":30}'
```

## Architecture

### serde_toon2 Crate

Core library providing TOON format serialization/deserialization:

- **error.rs**: Strongly-typed error handling with `ErrorKind` enum covering all TOON-specific errors (InvalidSyntax, IndentationError, CountMismatch, etc.). Errors include line/column information for debugging.

- **ser.rs**: Serializer implementation that converts Rust types to TOON format. Handles delimiter scoping, string quoting rules, number formatting, array headers with field lists, and indentation management.

- **de.rs**: Deserializer implementation that parses TOON into Rust types. Uses a two-phase approach: tokenization (line parsing with depth tracking) followed by recursive descent parsing. Supports strict mode validation and path expansion.

- **options.rs**: Configuration types:
  - `EncoderOptions`: indent size, delimiter (comma/tab/pipe), key folding, flatten depth
  - `DecoderOptions`: indent size, strict mode, path expansion
  - `Delimiter`: enum for array row delimiters (comma, tab, pipe)

- **value.rs**: TOON value representation (similar to serde_json::Value)

### CLI Crate

Simple converter tool (cli/src/main.rs) that:
1. Accepts input as file path, URL, or raw string
2. Auto-detects JSON vs TOON format
3. Converts between formats and outputs to stdout

### Test Strategy

Tests are fixture-based using JSON test definitions in `serde_toon2/tests/fixtures/`:
- `encode/`: JSON → TOON conversion tests
- `decode/`: TOON → JSON conversion tests

Each fixture file contains a `version` field and array of `tests` with:
- `name`: test description
- `input`: source data
- `expected`: expected output
- `shouldError`: whether test should fail
- `specSection`: reference to SPEC.md section

## Key Implementation Requirements

- **Zero-copy parsing**: Use borrowed string slices where possible in deserializer
- **SIMD vectorization**: Use SIMD for parsing hot paths (delimiter detection, whitespace handling)
- **Strongly-typed errors**: Always use `ErrorKind` enum variants, never generic strings
- **Strict spec compliance**: Reference SPEC.md for delimiter rules, quoting rules, indentation requirements
- **2025 Rust best practices**: Use latest stable Rust (edition 2024 for CLI, 2021 for library), avoid deprecated patterns

## TOON Format Key Concepts

- **Indentation-based structure**: Objects use indentation instead of braces (like YAML)
- **Array headers**: Declare length and optional field list: `[3]` or `[3 name,age]`
- **Delimiter scoping**: Arrays can use comma, tab, or pipe delimiters; delimiter is scoped to array
- **Minimal quoting**: Strings quoted only when necessary (contains delimiters, reserved words, etc.)
- **Line-oriented**: Each logical unit on its own line; no multi-line values except via escapes

## Specification Reference

Full TOON specification is in SPEC.md (large file). Key sections:
- Section 6: Header syntax
- Section 7: String/key quoting rules
- Section 9: Array format
- Section 11: Delimiter rules
- Section 12: Indentation rules
- Section 14: Strict mode validation errors
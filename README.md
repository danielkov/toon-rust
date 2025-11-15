# toon

Command-line tool for converting between JSON/YAML and TOON (Token-Oriented Object Notation) formats.

## Installation

```sh
cargo install --git https://github.com/danielkov/toon-rust
```

## Usage

The CLI provides two subcommands: `encode` (JSON/YAML → TOON) and `decode` (TOON → JSON/YAML).

### Encode JSON/YAML to TOON

```sh
toon encode [OPTIONS] <INPUT>
toon e [OPTIONS] <INPUT>  # short alias
```

**Options:**

- `--delimiter <comma|tab|pipe>` - Array element delimiter (default: comma)
- `--indent <NUM>` - Spaces per indentation level (default: 2)
- `--key-folding <off|safe>` - Key folding mode (default: off)
- `--flatten-depth <NUM>` - Maximum depth for inlining nested structures

**Examples:**

```sh
# Convert JSON file to TOON
toon encode data.json

# Convert YAML file to TOON
toon encode data.yaml

# Fetch and convert JSON from URL
toon encode https://api.github.com/users

# Convert inline JSON string
toon e '{"name":"Alice","age":30}'

# Use pipe delimiter for arrays
toon encode --delimiter pipe data.json
```

### Decode TOON to JSON/YAML

```sh
toon decode [OPTIONS] <INPUT>
toon d [OPTIONS] <INPUT>  # short alias
```

**Options:**

- `--indent <NUM>` - Spaces per indentation level (default: 2)
- `--strict` - Enable strict validation mode
- `--expand-paths <off|safe>` - Path expansion mode (default: off)
- `-o, --output-type <json|yaml>` - Output format (default: json)

**Examples:**

```sh
# Convert TOON file to JSON
toon decode data.toon

# Convert TOON to YAML
toon decode --output-type yaml data.toon
toon d -o yaml data.toon

# Convert inline TOON string
toon d 'name Alice age 30'

# Enable strict validation
toon decode --strict data.toon
```

## Input Sources

The tool accepts three types of input:

- **File paths** - Reads from local filesystem
- **URLs** - Fetches content via HTTP/HTTPS GET request
- **Raw strings** - Parses string directly from command line argument

## Output

All output is written to stdout. Pipe to a file or other tools as needed:

```sh
toon encode data.json > output.toon
toon decode data.toon | jq .
```

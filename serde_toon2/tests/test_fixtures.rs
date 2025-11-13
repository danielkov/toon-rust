use std::{fs::File, io::Read, path::Path};

use datatest_stable::Result;
use serde::Deserialize;
use serde_toon2::{DecoderOptions, Delimiter, EncoderOptions, KeyFolding, PathExpansion};

#[derive(Deserialize)]
struct DecodeTestOptions {
    #[serde(rename = "expandPaths")]
    expand_paths: Option<String>,
    strict: Option<bool>,
    indent: Option<usize>,
}

impl DecodeTestOptions {
    fn to_decoder_options(&self) -> DecoderOptions {
        let mut opts = DecoderOptions::default();

        if let Some(ref expand_paths) = self.expand_paths {
            opts.expand_paths = match expand_paths.as_str() {
                "safe" => PathExpansion::Safe,
                "off" => PathExpansion::Off,
                _ => PathExpansion::Off,
            };
        }

        if let Some(strict) = self.strict {
            opts.strict = strict;
        }

        if let Some(indent) = self.indent {
            opts.indent = indent;
        }

        opts
    }
}

#[derive(Deserialize)]
struct EncodeTestOptions {
    #[serde(rename = "keyFolding")]
    key_folding: Option<String>,
    #[serde(rename = "flattenDepth")]
    flatten_depth: Option<usize>,
    delimiter: Option<String>,
    indent: Option<usize>,
}

impl EncodeTestOptions {
    fn to_encoder_options(&self) -> EncoderOptions {
        let mut opts = EncoderOptions::default();

        if let Some(ref key_folding) = self.key_folding {
            opts.key_folding = match key_folding.as_str() {
                "safe" => KeyFolding::Safe,
                "off" => KeyFolding::Off,
                _ => KeyFolding::Off,
            };
        }

        if let Some(flatten_depth) = self.flatten_depth {
            opts.flatten_depth = flatten_depth;
        }

        if let Some(ref delimiter) = self.delimiter {
            opts.delimiter = match delimiter.as_str() {
                "," => Delimiter::Comma,
                "\t" => Delimiter::Tab,
                "|" => Delimiter::Pipe,
                _ => Delimiter::Comma,
            };
        }

        if let Some(indent) = self.indent {
            opts.indent = indent;
        }

        opts
    }
}

#[derive(Deserialize)]
struct DecodeTest {
    name: String,
    input: String,
    expected: serde_json::Value,
    #[serde(rename = "shouldError", default)]
    should_error: bool,
    options: Option<DecodeTestOptions>,
    #[serde(rename = "specSection")]
    spec_section: String,
}

#[derive(Deserialize)]
struct EncodeTest {
    name: String,
    input: serde_json::Value,
    expected: String,
    #[serde(rename = "shouldError", default)]
    should_error: bool,
    options: Option<EncodeTestOptions>,
    #[serde(rename = "specSection")]
    spec_section: String,
}

#[derive(Deserialize)]
#[serde(bound = "T: serde::de::DeserializeOwned")]
struct Fixture<T: serde::de::DeserializeOwned> {
    tests: Vec<T>,
}

fn test_encode_fixture(path: &Path) -> Result<()> {
    let mut file = File::open(path)?;
    let mut json_string = String::new();
    file.read_to_string(&mut json_string)?;

    let fixture: Fixture<EncodeTest> = serde_json::from_str(&json_string)?;

    for test in fixture.tests {
        let result = if let Some(options) = test.options {
            let opts = options.to_encoder_options();
            serde_toon2::to_string_with_options(&test.input, opts)
        } else {
            serde_toon2::to_string(&test.input)
        };

        if test.should_error {
            assert!(
                result.is_err(),
                "expected error but got success: fixture: {}, spec: {}",
                test.name,
                test.spec_section
            );
        } else {
            let output = result.expect(&format!(
                "encode failed: fixture: {}, spec: {}",
                test.name, test.spec_section
            ));
            assert_eq!(
                output, test.expected,
                "result does not match expected: {}, spec: {}",
                test.name, test.spec_section
            );
        }
    }

    Ok(())
}

fn test_decode_fixture(path: &Path) -> Result<()> {
    let mut file = File::open(path)?;
    let mut json_string = String::new();
    file.read_to_string(&mut json_string)?;

    let fixture: Fixture<DecodeTest> = serde_json::from_str(&json_string)?;

    for test in fixture.tests {
        let result = if let Some(options) = test.options {
            let opts = options.to_decoder_options();
            serde_toon2::from_str_with_options(&test.input, opts)
        } else {
            serde_toon2::from_str(&test.input)
        };

        if test.should_error {
            assert!(
                result.is_err(),
                "expected error but got success: fixture {}, spec: {}",
                test.name,
                test.spec_section
            );
        } else {
            let output: serde_json::Value = result.expect(&format!(
                "decode failed: fixture {}, spec: {}",
                test.name, test.spec_section
            ));
            assert_eq!(
                output, test.expected,
                "result does not match expected: {}, spec: {}",
                test.name, test.spec_section
            );
        }
    }

    Ok(())
}

datatest_stable::harness! {
    { test = test_encode_fixture, root = "tests/fixtures/encode", pattern = r"^.*\.json$" },
    { test = test_decode_fixture, root = "tests/fixtures/decode", pattern = r"^.*\.json$" },
}

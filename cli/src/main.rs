use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use serde_toon2::{DecoderOptions, Delimiter, EncoderOptions, KeyFolding, PathExpansion};
use std::path::Path;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DelimiterArg {
    Comma,
    Tab,
    Pipe,
}

impl From<DelimiterArg> for Delimiter {
    fn from(arg: DelimiterArg) -> Self {
        match arg {
            DelimiterArg::Comma => Delimiter::Comma,
            DelimiterArg::Tab => Delimiter::Tab,
            DelimiterArg::Pipe => Delimiter::Pipe,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum KeyFoldingArg {
    Off,
    Safe,
}

impl From<KeyFoldingArg> for KeyFolding {
    fn from(arg: KeyFoldingArg) -> Self {
        match arg {
            KeyFoldingArg::Off => KeyFolding::Off,
            KeyFoldingArg::Safe => KeyFolding::Safe,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PathExpansionArg {
    Off,
    Safe,
}

impl From<PathExpansionArg> for PathExpansion {
    fn from(arg: PathExpansionArg) -> Self {
        match arg {
            PathExpansionArg::Off => PathExpansion::Off,
            PathExpansionArg::Safe => PathExpansion::Safe,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "toon")]
#[command(about = "Convert between JSON and TOON formats", long_about = None)]
#[command(after_help = "\x1b[1;4mExamples:\x1b[0m
  Convert JSON from a URL to TOON format:
    \x1b[1mtoon encode\x1b[0m https://api.github.com/users

  Convert a local JSON file to TOON:
    \x1b[1mtoon e\x1b[0m data.json

  Convert a local TOON file to JSON:
    \x1b[1mtoon decode\x1b[0m data.toon

  Convert raw JSON string to TOON:
    \x1b[1mtoon e\x1b[0m '{\"name\":\"Alice\",\"age\":30}'

  Convert raw TOON string to JSON:
    \x1b[1mtoon d\x1b[0m 'name Alice age 30'

  Use pipe delimiter when converting to TOON:
    \x1b[1mtoon encode\x1b[0m --delimiter pipe data.json

  Enable strict mode when parsing TOON:
    \x1b[1mtoon decode\x1b[0m --strict data.toon")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(alias = "e", about = "Encode JSON to TOON format")]
    Encode {
        #[arg(help = "Input source: file path, URL, or raw JSON string")]
        input: String,

        #[arg(
            long,
            value_enum,
            help = "Delimiter for array elements",
            default_value = "comma"
        )]
        delimiter: DelimiterArg,

        #[arg(
            long,
            help = "Number of spaces per indentation level",
            default_value = "2"
        )]
        indent: usize,

        #[arg(long, value_enum, help = "Key folding mode", default_value = "off")]
        key_folding: KeyFoldingArg,

        #[arg(long, help = "Maximum depth for inlining nested structures")]
        flatten_depth: Option<usize>,
    },

    #[command(alias = "d", about = "Decode TOON to JSON format")]
    Decode {
        #[arg(help = "Input source: file path, URL, or raw TOON string")]
        input: String,

        #[arg(
            long,
            help = "Number of spaces per indentation level",
            default_value = "2"
        )]
        indent: usize,

        #[arg(long, help = "Enable strict validation mode")]
        strict: bool,

        #[arg(long, value_enum, help = "Path expansion mode", default_value = "off")]
        expand_paths: PathExpansionArg,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = process(&cli).await {
        eprintln!("Error: {}\n", e);
        Cli::command().print_help().unwrap();
        std::process::exit(1);
    }
}

async fn get_input_content(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    if source.starts_with("http://") || source.starts_with("https://") {
        download_from_url(source).await
    } else if Path::new(source).exists() {
        read_from_file(source)
    } else {
        Ok(source.to_string())
    }
}

async fn download_from_url(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .user_agent("toon-cli/0.1.0")
        .build()?;
    let response = client.get(url).send().await?;
    let content = response.text().await?;
    Ok(content)
}

fn read_from_file(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}

async fn process(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    match &cli.command {
        Command::Encode {
            input,
            delimiter,
            indent,
            key_folding,
            flatten_depth,
        } => {
            let content = get_input_content(input).await?;
            let json: serde_json::Value = serde_json::from_str(&content)?;

            let encoder_opts = EncoderOptions {
                indent: *indent,
                delimiter: (*delimiter).into(),
                key_folding: (*key_folding).into(),
                flatten_depth: flatten_depth.unwrap_or(usize::MAX),
            };

            let toon_str = serde_toon2::to_string_with_options(&json, encoder_opts)?;
            print!("{}", toon_str);
        }
        Command::Decode {
            input,
            indent,
            strict,
            expand_paths,
        } => {
            let content = get_input_content(input).await?;

            let decoder_opts = DecoderOptions {
                indent: *indent,
                strict: *strict,
                expand_paths: (*expand_paths).into(),
            };

            let value: serde_json::Value =
                serde_toon2::from_str_with_options(&content, decoder_opts)?;
            let json_str = serde_json::to_string_pretty(&value)?;
            print!("{}", json_str);
        }
    }

    Ok(())
}

use clap::{CommandFactory, Parser};
use std::path::Path;

#[derive(Debug, Parser)]
#[command(name = "toon")]
#[command(about = "Auto-convert between JSON and TOON formats", long_about = None)]
#[command(after_help = "\x1b[1;4mExamples:\x1b[0m
  Convert JSON from a URL to TOON format:
    \x1b[1mtoon\x1b[0m https://api.github.com/users

  Convert a local JSON file to TOON:
    \x1b[1mtoon\x1b[0m data.json

  Convert a local TOON file to JSON:
    \x1b[1mtoon\x1b[0m data.toon

  Convert raw JSON string to TOON:
    \x1b[1mtoon\x1b[0m '{\"name\":\"Alice\",\"age\":30}'

  Convert raw TOON string to JSON:
    \x1b[1mtoon\x1b[0m 'name Alice age 30'")]
struct Cli {
    #[arg(help = "Input source: file path, URL, or raw JSON/TOON string")]
    input: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = process(&cli.input).await {
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

async fn process(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = get_input_content(input).await?;

    match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(json) => {
            let toon_str = serde_toon::to_string(&json)?;
            print!("{}", toon_str);
        }
        Err(json_err) => match serde_toon::from_str::<serde_json::Value>(&content) {
            Ok(value) => {
                let json_str = serde_json::to_string_pretty(&value)?;
                print!("{}", json_str);
            }
            Err(toon_err) => {
                return Err(format!(
                    "Input is neither valid JSON nor valid TOON format\nJSON error: {}\nTOON error: {}",
                    json_err, toon_err
                )
                .into());
            }
        },
    }

    Ok(())
}

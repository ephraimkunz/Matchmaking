mod matching;
mod parsing;

use anyhow::Result;

const RESPONSE_FILE_NAME: &str = "responses.csv";

fn main() -> Result<()> {
    let responses = parsing::parse_responses(RESPONSE_FILE_NAME)?;
    let matches = matching::create_matches(&responses)?;
    println!("{matches}");

    Ok(())
}

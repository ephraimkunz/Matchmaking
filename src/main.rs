mod matching;
mod parsing;

use anyhow::Result;

const RESPONSE_FILE_NAME: &str = "responses.csv";

fn main() -> Result<()> {
    let mut reader = csv::Reader::from_path(RESPONSE_FILE_NAME)?;
    let responses = parsing::parse_responses(&mut reader)?;
    let matches = matching::create_matches(responses)?;
    println!("{matches}");

    Ok(())
}

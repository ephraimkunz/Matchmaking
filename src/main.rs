#![warn(clippy::pedantic)]
#![warn(clippy::correctness)]
#![warn(clippy::suspicious)]
#![warn(clippy::complexity)]
#![warn(clippy::perf)]
#![warn(clippy::style)]
#![allow(clippy::cast_precision_loss)]

mod matching;
mod parsing;

use anyhow::Result;
use clap::Parser;
use matching::Matches;
use std::path::PathBuf;

/// Generate shortlists of compatible dating partners, based on input dating questionnaire.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the input file
    #[arg(value_name = "INPUT_FILE")]
    input_file_name: PathBuf,

    /// Sorts each shortlist by descending score, so best matches appear at the top. If this is not set, the order of each shortlist is random.
    #[arg(short, long)]
    sort_shortlists_by_score: bool,

    /// Print match score next to each match name
    #[arg(short, long)]
    print_scores: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let matches = parse_and_generate_matches(&args)?;
    println!("{matches}");
    Ok(())
}

fn parse_and_generate_matches(args: &Args) -> Result<Matches> {
    let mut reader = csv::Reader::from_path(args.input_file_name.clone())?;
    let responses = parsing::parse_responses(&mut reader)?;
    let matches =
        matching::create_matches(&responses, args.sort_shortlists_by_score, args.print_scores);
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use itertools::Itertools;

    #[test]
    fn verify_cli() {
        Args::command().debug_assert();
    }

    #[test]
    fn parse_and_generate_matches_test() {
        let input = [
            "matchmaking",
            "--sort-shortlists-by-score",
            "--print-scores",
            "test_data/many_generated.csv",
        ];
        let args = Args::try_parse_from(input).unwrap();
        let matches = parse_and_generate_matches(&args).unwrap();
        let output = format!("{matches}");

        assert!(!output.is_empty());
        assert!(output.lines().collect_vec().len() > 100);
    }
}

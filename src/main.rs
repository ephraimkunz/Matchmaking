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
use matching::{Diagnostics, Matches};
use std::{io::Write, path::PathBuf};

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

    /// Print diagnostic statistics to stderr
    #[arg(short, long)]
    diagnostics: bool,

    /// The ideal length of each person's shortlist
    #[arg(short, long, default_value_t = 5)]
    target_shortlist: usize,

    /// The minimum length of each person's shortlist. The algorithm will strive for the target, but if it
    /// can't be satisfied it will relax the `max_appearances` and try to reach at least this length of shortlist. Should always be
    /// smaller than `target_shortlist`.
    #[arg(short, long, default_value_t = 3)]
    min_shortlist: usize,

    /// The maximum number of shortlists someone should appear on
    #[arg(short = 'a', long, default_value_t = 8)]
    max_appearances: usize,

    /// The relaxed number of shortlists someone should appear on. If the algorithm can't satisfy the target shortlist,
    /// it will relax this to try to at least reach shortlists of length `min_shortlist`. Should always be larger than `max_appearances`.
    #[arg(short = 'r', long, default_value_t = 10)]
    max_appearances_relaxed: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let (matches, diagnostics) = parse_and_generate_matches(&args)?;

    let out = matches.to_string();
    std::io::stdout().lock().write_all(out.as_bytes())?;

    if let Some(diag) = diagnostics {
        write!(std::io::stderr().lock(), "{diag}")?;
    }
    Ok(())
}

fn parse_and_generate_matches(args: &Args) -> Result<(Matches, Option<Diagnostics>)> {
    let mut reader = csv::Reader::from_path(&args.input_file_name)?;
    let responses = parsing::parse_responses(&mut reader)?;
    let result = matching::create_matches(
        &responses,
        args.sort_shortlists_by_score,
        args.print_scores,
        args.diagnostics,
        args.target_shortlist,
        args.min_shortlist,
        args.max_appearances,
        args.max_appearances_relaxed,
    );
    Ok(result)
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
        let (matches, _) = parse_and_generate_matches(&args).unwrap();
        let output = format!("{matches}");

        assert!(!output.is_empty());
        assert!(output.lines().collect_vec().len() > 100);
    }
}

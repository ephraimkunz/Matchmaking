#![warn(clippy::pedantic)]
#![warn(clippy::correctness)]
#![warn(clippy::suspicious)]
#![warn(clippy::complexity)]
#![warn(clippy::perf)]
#![warn(clippy::style)]
#![allow(clippy::cast_precision_loss)]

use anyhow::Result;
use clap::Parser;
use matchmaking::parse_and_generate_matches;
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

    /// The maximum number of shortlists someone should appear on
    #[arg(short = 'a', long, default_value_t = 8)]
    max_appearances: usize,

    /// The maximum number of shortlists someone can appear on when the pool is tight. The algorithm raises
    /// the appearance cap one at a time from `max_appearances` up to this limit whenever a pass stalls, always
    /// continuing to fill toward `target_shortlist`. Should always be larger than `max_appearances`.
    #[arg(short = 'r', long, default_value_t = 10)]
    max_appearances_relaxed: usize,

    /// The random seed to use for reproducible results. If one is not provided, the rng will be seeded randomly
    /// (but the seed will still be output along with the diagnostics).
    #[arg(long)]
    seed: Option<u64>,

    /// If provided, the person with this id (email)'s shortlist is printed.
    #[arg(long, value_name = "PERSON_ID")]
    debug_print_candidate_list: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let (matches, diagnostics) = parse_and_generate_matches(
        args.input_file_name,
        args.seed,
        args.sort_shortlists_by_score,
        args.print_scores,
        args.diagnostics,
        args.target_shortlist,
        args.max_appearances,
        args.max_appearances_relaxed,
        args.debug_print_candidate_list,
    )?;

    let out = matches.to_string();
    std::io::stdout().lock().write_all(out.as_bytes())?;

    if let Some(diag) = diagnostics {
        write!(std::io::stderr().lock(), "{diag}")?;
    }
    Ok(())
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
        let (matches, _) = parse_and_generate_matches(
            args.input_file_name,
            args.seed,
            args.sort_shortlists_by_score,
            args.print_scores,
            args.diagnostics,
            args.target_shortlist,
            args.max_appearances,
            args.max_appearances_relaxed,
            args.debug_print_candidate_list,
        )
        .unwrap();
        let output = format!("{matches}");

        assert!(!output.is_empty());
        assert!(output.lines().collect_vec().len() > 100);
    }
}

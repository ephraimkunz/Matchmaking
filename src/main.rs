#![warn(clippy::pedantic)]
#![warn(clippy::correctness)]
#![warn(clippy::suspicious)]
#![warn(clippy::complexity)]
#![warn(clippy::perf)]
#![warn(clippy::style)]
#![allow(clippy::cast_precision_loss)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use itertools::Itertools;
use matchmaking::Matches;
use matchmaking::generate_docx;
use matchmaking::parse_and_generate_matches;
use matchmaking::validate_ids;
use std::io::Write;
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

    /// Print match score next to each match name. Only applies if output-format is plain-text.
    #[arg(short, long)]
    print_scores: bool,

    /// Print diagnostic statistics to stderr
    #[arg(short, long)]
    diagnostics: bool,

    /// The ideal length of each person's shortlist
    #[arg(long, default_value_t = 5)]
    target_shortlist: usize,

    /// The maximum number of shortlists someone should appear on
    #[arg(long, default_value_t = 8)]
    max_appearances: usize,

    /// The maximum number of shortlists someone can appear on when the pool is tight. The algorithm raises
    /// the appearance cap one at a time from `max_appearances` up to this limit whenever a pass stalls, always
    /// continuing to fill toward `target_shortlist`. Should always be larger than `max_appearances`.
    #[arg(long, default_value_t = 10)]
    max_appearances_relaxed: usize,

    /// The random seed to use for reproducible results. If one is not provided, the rng will be seeded randomly
    /// (but the seed will still be output along with the diagnostics).
    #[arg(long)]
    seed: Option<u64>,

    /// If provided, the person with this id (email)'s candidate list is printed to stdout.
    #[arg(long, value_name = "PERSON_ID")]
    debug_print_candidate_list: Option<String>,

    /// What type of output to generate.
    #[arg(short, long, value_enum, default_value_t = OutputFormat::PlainText)]
    output_format: OutputFormat,

    /// Ids (emails) to exclude during parsing. Any id provided here is as if it was never in the input to begin with.
    /// This is helpful when people live far away and you plan to run multiple matching rounds and want to exclude on some.
    #[arg(long, default_values_t = Vec::<String>::new())]
    excluded_ids: Vec<String>,

    /// Ids (emails) to output. If empty, all non-excluded ids are output. If present, only the provided ids are output.
    /// This is helpful when people live far away and you plan to run multiple matching rounds and want only output a
    /// few ids on some.
    #[arg(long, default_values_t = Vec::<String>::new(), num_args(1..))]
    output_ids: Vec<String>,
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    /// Plaintext for checking results is printed to stdout
    PlainText,
    /// Word document names matches.docx is created and opened
    DocX,
    /// JSON document is printed to stdout
    Json,
}

fn main() -> Result<()> {
    let args = Args::parse();
    run(args, &mut std::io::stdout(), &mut std::io::stderr())
}

fn run<W1: Write, W2: Write>(args: Args, stdout: &mut W1, stderr: &mut W2) -> Result<()> {
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
        &args.excluded_ids,
    )?;

    let output_ids = validate_ids(
        &args.output_ids,
        matches.cards.iter().map(|c| c.email.as_str()),
    )
    .with_context(|| "Failed to parse output_ids")?;

    let filtered_cards = matches
        .cards
        .into_iter()
        .filter(|c| {
            if output_ids.is_empty() {
                true
            } else {
                output_ids.contains(&c.email)
            }
        })
        .collect_vec();
    let matches = Matches {
        cards: filtered_cards,
        ..matches
    };

    match args.output_format {
        OutputFormat::PlainText => {
            let out = matches.to_string();
            write!(stdout, "{out}")?;
        }
        OutputFormat::DocX => {
            let path = generate_docx(&matches)?;
            open::that(path)?;
        }
        OutputFormat::Json => write!(stdout, "{}", serde_json::to_string_pretty(&matches)?)?,
    }

    if let Some(diag) = diagnostics {
        write!(stderr, "{diag}")?;
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
    fn basic() {
        let input = [
            "matchmaking",
            "--sort-shortlists-by-score",
            "--print-scores",
            "test_data/many_generated.csv",
        ];
        let args = Args::try_parse_from(input).unwrap();
        let mut stdout = vec![];
        let mut stderr = vec![];
        assert!(run(args, &mut stdout, &mut stderr).is_ok());

        assert!(!stdout.is_empty());
        assert!(stderr.is_empty());

        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.lines().collect_vec().len() > 100);
    }

    #[test]
    fn diagnostic_info() {
        let input = [
            "matchmaking",
            "--sort-shortlists-by-score",
            "--print-scores",
            "-d",
            "test_data/many_generated.csv",
        ];
        let args = Args::try_parse_from(input).unwrap();
        let mut stdout = vec![];
        let mut stderr = vec![];
        assert!(run(args, &mut stdout, &mut stderr).is_ok());

        assert!(!stdout.is_empty());
        assert!(!stderr.is_empty());

        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.lines().collect_vec().len() > 100);

        let stderr = String::from_utf8(stderr).unwrap();
        let stderr_lines = stderr.lines().collect_vec().len();
        assert!(stderr_lines > 50 && stderr_lines < 100);
    }

    #[test]
    fn output_ids_flag_no_args() {
        let input = [
            "matchmaking",
            "test_data/many_generated.csv",
            "--output-ids",
            "-o",
            "plain-text",
        ];

        // try_parse_from returns a Result, preventing test panics
        let parsed = Args::try_parse_from(input);

        assert!(parsed.is_err());
    }

    #[test]
    fn output_ids_limits() {
        let input = [
            "matchmaking",
            "test_data/many_generated.csv",
            "--seed",
            "1",
            "--output-ids",
            "aurora.green@example.com",
            "spencer.morris@example.com",
        ];
        let args = Args::try_parse_from(input).unwrap();
        let mut stdout = vec![];
        let mut stderr = vec![];
        assert!(run(args, &mut stdout, &mut stderr).is_ok());

        assert!(!stdout.is_empty());
        assert!(stderr.is_empty());

        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.lines().collect_vec().len() < 70);
    }

    #[test]
    fn output_ids_invalid() {
        let input = [
            "matchmaking",
            "test_data/many_generated.csv",
            "--seed",
            "1",
            "--output-ids",
            "aurora.green@example.com",
            "abc",
            "def",
        ];
        let args = Args::try_parse_from(input).unwrap();
        let mut stdout = vec![];
        let mut stderr = vec![];
        let result = run(args, &mut stdout, &mut stderr);
        assert!(result.is_err());
    }

    #[test]
    fn output_ids_non_existent() {
        let input = [
            "matchmaking",
            "test_data/many_generated.csv",
            "--seed",
            "1",
            "--output-ids",
            "aurora.green@example.com",
            "spencer.joe@example.com",
            "spencer.joe_3@example.com",
        ];
        let args = Args::try_parse_from(input).unwrap();
        let mut stdout = vec![];
        let mut stderr = vec![];
        let result = run(args, &mut stdout, &mut stderr);
        assert!(result.is_err());
    }
}

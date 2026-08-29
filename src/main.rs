use anyhow::Result;
use clap::{Parser, ValueEnum};
use matchmaking::generate_docx;
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

    /// Print match score next to each match name. Only applies if output-format is plain-text.
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

    /// If provided, the person with this id (email)'s shortlist is printed to stdout.
    #[arg(long, value_name = "PERSON_ID")]
    debug_print_candidate_list: Option<String>,

    /// What type of output to generate.
    #[arg(short, long, value_enum, default_value_t = OutputFormat::PlainText)]
    output_format: OutputFormat,
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

    match args.output_format {
        OutputFormat::PlainText => {
            let out = matches.to_string();
            std::io::stdout().lock().write_all(out.as_bytes())?;
        }
        OutputFormat::DocX => {
            let path = generate_docx(&matches)?;
            open::that(path)?;
        }
        OutputFormat::Json => print!("{}", serde_json::to_string_pretty(&matches)?),
    }

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

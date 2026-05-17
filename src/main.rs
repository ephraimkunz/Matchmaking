mod matching;
mod parsing;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

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

    /// A seed for the RNG, used if you want deterministic results from run to run. If not supplied, RNG is seeded directly from OS.
    #[arg(short, long)]
    rng_seed: Option<u64>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut reader = csv::Reader::from_path(args.input_file_name)?;
    let responses = parsing::parse_responses(&mut reader)?;
    let matches = matching::create_matches(responses, args.sort_shortlists_by_score, args.rng_seed)?;
    println!("{matches}");

    Ok(())
}

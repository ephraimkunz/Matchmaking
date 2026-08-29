#![warn(clippy::pedantic)]
#![warn(clippy::correctness)]
#![warn(clippy::suspicious)]
#![warn(clippy::complexity)]
#![warn(clippy::perf)]
#![warn(clippy::style)]
#![allow(clippy::cast_precision_loss)]

use std::path::PathBuf;

use anyhow::Result;
use diagnostics::Diagnostics;

use rand::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod diagnostics;
mod docx;
mod matching;
mod parsing;
mod validation;

pub use docx::generate_docx;
pub use matching::Matches;
pub use validation::validate_ids;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn generate_matches_from_csv_text(
    csv_text: &str,
    sort_shortlists_by_score: bool,
    print_scores: bool,
    collect_diagnostics: bool,
    target_shortlist: usize,
    max_appearances: usize,
    max_appearances_relaxed: usize,
    rng_seed: Option<u64>,
) -> Result<JsValue, JsValue> {
    let (mut rng, seed) = rng_and_seed(rng_seed);

    // Parse CSV from string instead of file path
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let responses = parsing::parse_responses(&mut reader, &mut rng, &[])
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let (matches, diagnostics) = matching::create_matches(
        &responses,
        &mut rng,
        seed,
        sort_shortlists_by_score,
        print_scores,
        collect_diagnostics,
        target_shortlist,
        max_appearances,
        max_appearances_relaxed,
        None,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Use Display impl you already have — no new serialization needed
    let output = MatchOutput {
        matches_text: matches.to_string(),
        diagnostics_text: diagnostics.map(|d| d.to_string()),
    };

    serde_wasm_bindgen::to_value(&output).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[derive(serde::Serialize)]
struct MatchOutput {
    matches_text: String,
    diagnostics_text: Option<String>,
}

/// # Errors
///
/// Returns an error if `input_filename` can't be opened, or if its contents don't parse
/// as a valid questionnaire response CSV (see [`parsing::parse_responses`]).
#[allow(clippy::too_many_arguments)]
pub fn parse_and_generate_matches(
    input_filename: PathBuf,
    rng_seed: Option<u64>,
    sort_shortlists_by_score: bool,
    print_scores: bool,
    collect_diagnostics: bool,
    target_shortlist: usize,
    max_appearances: usize,
    max_appearances_relaxed: usize,
    debug_print_candidate_list: Option<String>,
    excluded_ids: &[String],
) -> Result<(Matches, Option<Diagnostics>)> {
    let (mut rng, seed) = rng_and_seed(rng_seed);
    let mut reader = csv::Reader::from_path(input_filename)?;
    let responses = parsing::parse_responses(&mut reader, &mut rng, excluded_ids)?;
    matching::create_matches(
        &responses,
        &mut rng,
        seed,
        sort_shortlists_by_score,
        print_scores,
        collect_diagnostics,
        target_shortlist,
        max_appearances,
        max_appearances_relaxed,
        debug_print_candidate_list,
    )
}

#[must_use]
pub fn rng_and_seed(rng_seed: Option<u64>) -> (StdRng, u64) {
    let (rng, seed) = if let Some(seed) = rng_seed {
        (StdRng::seed_from_u64(seed), seed)
    } else {
        let seed = rand::rng().next_u64();
        (StdRng::seed_from_u64(seed), seed)
    };

    (rng, seed)
}

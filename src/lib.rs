use std::path::PathBuf;

use anyhow::Result;
use diagnostics::Diagnostics;
use matching::Matches;

use rand::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod diagnostics;
mod matching;
mod parsing;

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
    let responses = parsing::parse_responses(&mut reader, &mut rng)
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
    );

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
) -> Result<(Matches, Option<Diagnostics>)> {
    let (mut rng, seed) = rng_and_seed(rng_seed);

    let mut reader = csv::Reader::from_path(input_filename)?;
    let responses = parsing::parse_responses(&mut reader, &mut rng)?;
    let result = matching::create_matches(
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
    );
    Ok(result)
}

pub fn rng_and_seed(rng_seed: Option<u64>) -> (StdRng, u64) {
    let (rng, seed) = if let Some(seed) = rng_seed {
        (StdRng::seed_from_u64(seed), seed)
    } else {
        let seed = rand::rng().next_u64();
        (StdRng::seed_from_u64(seed), seed)
    };

    (rng, seed)
}

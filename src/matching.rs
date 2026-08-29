use itertools::Itertools;
use rand::prelude::*;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use serde::Serialize;
use std::fmt::Write;

use crate::parsing::{
    Age, FreeResponse, Gender, MarriageTimelineResponse, PartnersReligionResponse,
    QuestionnaireResponse, YesNoMaybeResponse,
};

use crate::diagnostics::{
    DealbreakerCause, Diagnostics, PairsStats, ShortlistStats, build_diagnostics,
};
use crate::validation::validate_id;

use anyhow::{Context, Result};

#[derive(Clone, Debug, PartialEq, Serialize)]

pub struct Matches(pub Vec<MatchCard>);

impl Matches {
    /// # Errors
    ///
    /// Returns errors if writeln! fails.
    pub fn plaintext(&self, print_scores: bool) -> Result<String> {
        let mut output = String::new();
        for card in &self.0 {
            writeln!(output, "{} ({})\n\nMatches:", card.name, card.email)?;
            for (index, m) in card.shortlist.iter().enumerate() {
                write!(output, "{}", m.plaintext(print_scores)?)?;

                if index < (card.shortlist.len() - 1) {
                    writeln!(output)?;
                }
            }

            writeln!(
                output,
                "\n========================================================================\n"
            )?;
        }
        Ok(output)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MatchCard {
    pub name: String,
    pub email: String,
    pub shortlist: Vec<ShortlistMatch>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ShortlistMatch {
    pub name: String,
    pub age: Age,
    pub email: String,
    pub freeresponse: FreeResponse,
    pub score: f32,
}

impl ShortlistMatch {
    pub fn plaintext(&self, print_scores: bool) -> Result<String> {
        let mut output = String::new();
        if print_scores {
            writeln!(
                output,
                "  {} - {} ({}) ({})",
                self.name, self.age.0, self.email, self.score
            )?;
        } else {
            writeln!(output, "  {} - {} ({})", self.name, self.age.0, self.email)?;
        }

        for (k, v) in &self.freeresponse.responses {
            writeln!(output, "    {k} {v}")?;
        }
        Ok(output)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_matches(
    responses: &[QuestionnaireResponse],
    rng: &mut StdRng,
    rng_seed: u64,
    sort_shortlists_by_score: bool,
    collect_diagnostics: bool,
    target_shortlist: usize,
    max_appearances: usize,
    max_appearances_relaxed: usize,
    debug_print_candidate_list: Option<String>,
) -> Result<(Matches, Option<Diagnostics>)> {
    // Score all pairs
    let (pairs, pairs_stats) = build_scored_pairs(responses, collect_diagnostics);

    let ids = responses
        .iter()
        .map(QuestionnaireResponse::id)
        .collect_vec();

    // Deterministic prep, shared by the randomized assignment below and the diagnostics.
    let ranked_candidates = build_ranked_candidates(&ids, &pairs);

    if let Some(id) = debug_print_candidate_list {
        let id = validate_id(&id, ids.iter().copied())
            .with_context(|| "Failed to parse debug_print_candidate_list PERSON_ID")?;
        match ranked_candidates.get(id.as_str()) {
            Some(candidates) => eprintln!("{id}'s candidate_list{candidates:?}"),
            None => eprintln!("No participant with id \"{id}\" was found in the input."),
        }
    }

    // Assign shortlists via round-robin
    let (shortlists, shortlist_stats) = assign_shortlists(
        &ids,
        &ranked_candidates,
        rng,
        collect_diagnostics,
        target_shortlist,
        max_appearances,
        max_appearances_relaxed,
    );

    let ids_to_responses: FxHashMap<&str, &QuestionnaireResponse> =
        responses.iter().map(|r| (r.id(), r)).collect();

    let mut matches = shortlists
        .into_iter()
        .map(|(id, matches)| {
            let response = ids_to_responses
                .get(id.as_str())
                .expect("Can't find response for id");

            let matches = if sort_shortlists_by_score {
                let mut matches = matches;
                matches.sort_unstable_by(|a, b| {
                    b.1.partial_cmp(&a.1)
                        .expect("Should be able to compare floats")
                });
                matches
            } else {
                let mut matches = matches;
                matches.shuffle(rng);
                matches
            };
            MatchCard {
                name: response.demographics.name.clone(),
                email: response.demographics.email.clone(),
                shortlist: matches
                    .into_iter()
                    .map(|(matched_id, matched_score)| {
                        let match_response = ids_to_responses
                            .get(matched_id.as_str())
                            .expect("Can't find response for id");
                        ShortlistMatch {
                            name: match_response.demographics.name.clone(),
                            age: match_response.demographics.age.clone(),
                            email: match_response.demographics.email.clone(),
                            freeresponse: match_response.freeresponse.clone(),
                            score: matched_score,
                        }
                    })
                    .collect_vec(),
            }
        })
        .collect_vec();

    matches.sort_unstable_by(|a, b| a.name.cmp(&b.name));

    let result = Matches(matches);

    let diagnostics = build_diagnostics(
        pairs_stats,
        shortlist_stats,
        &ids,
        &pairs,
        &ranked_candidates,
        &result,
        target_shortlist,
        rng_seed,
    );

    Ok((result, diagnostics))
}

/// Returns Ok(()) if no dealbreakers, or Err with the first cause found.
fn passes_dealbreakers(
    a: &QuestionnaireResponse,
    b: &QuestionnaireResponse,
) -> Result<(), DealbreakerCause> {
    match (
        &a.dealbreakers.wants_children,
        &b.dealbreakers.wants_children,
    ) {
        (YesNoMaybeResponse::No, YesNoMaybeResponse::Yes)
        | (YesNoMaybeResponse::Yes, YesNoMaybeResponse::No) => {
            return Err(DealbreakerCause::WantsChildren);
        }
        _ => (),
    }

    match (&a.dealbreakers.stay_local, &b.dealbreakers.stay_local) {
        (YesNoMaybeResponse::No, YesNoMaybeResponse::Yes)
        | (YesNoMaybeResponse::Yes, YesNoMaybeResponse::No) => {
            return Err(DealbreakerCause::StayLocal);
        }
        _ => (),
    }

    match (
        &a.dealbreakers.marriage_timeline,
        &b.dealbreakers.marriage_timeline,
    ) {
        (MarriageTimelineResponse::ZeroToTwo, MarriageTimelineResponse::FivePlus)
        | (MarriageTimelineResponse::FivePlus, MarriageTimelineResponse::ZeroToTwo) => {
            return Err(DealbreakerCause::MarriageTimeline);
        }
        _ => (),
    }

    match &a.dealbreakers.partners_religious_commitment {
        PartnersReligionResponse::Same
            if a.dealbreakers.my_religious_commitment.0
                != b.dealbreakers.my_religious_commitment.0 =>
        {
            return Err(DealbreakerCause::Religion);
        }
        PartnersReligionResponse::Within1Level
            if a.dealbreakers
                .my_religious_commitment
                .0
                .abs_diff(b.dealbreakers.my_religious_commitment.0)
                > 1 =>
        {
            return Err(DealbreakerCause::Religion);
        }
        _ => (),
    }

    match &b.dealbreakers.partners_religious_commitment {
        PartnersReligionResponse::Same
            if b.dealbreakers.my_religious_commitment.0
                != a.dealbreakers.my_religious_commitment.0 =>
        {
            return Err(DealbreakerCause::Religion);
        }
        PartnersReligionResponse::Within1Level
            if b.dealbreakers
                .my_religious_commitment
                .0
                .abs_diff(a.dealbreakers.my_religious_commitment.0)
                > 1 =>
        {
            return Err(DealbreakerCause::Religion);
        }
        _ => (),
    }

    Ok(())
}

fn calculate_subject_chosen_weight_scale_factor(a: &QuestionnaireResponse) -> f32 {
    // Cap the subject-chosen weights. If they rated everything max importance, the average
    // would be 2.0, exceeding this cap. All the weights get scaled down proportionally.
    // For anti-gaming and preventing someone who feels strongly about everything from having
    // their scores behave differently from everyone else.
    const PERSON_BOOST_CAP: f32 = 1.5;
    let weights_count =
        a.corevalues.response_and_weights.len() + a.relationshipdynamics.response_and_weights.len();
    let weights_sum: f32 = a
        .corevalues
        .response_and_weights
        .iter()
        .map(|q| q.weight.normalized())
        .chain(
            a.relationshipdynamics
                .response_and_weights
                .iter()
                .map(|q| q.weight.normalized()),
        )
        .sum();
    let average_weight = weights_sum / weights_count as f32;
    if average_weight > PERSON_BOOST_CAP {
        PERSON_BOOST_CAP / average_weight
    } else {
        1.0
    }
}

fn process_core_values(
    a: &QuestionnaireResponse,
    b: &QuestionnaireResponse,
    subject_chosen_weight_scale_factor: f32,
) -> (f32, f32) {
    const CORE_VALUES_SECTION_WEIGHT: f32 = 1.0;
    let mut total = 0.0;
    let mut weight_sum = 0.0;
    for (a_answer, b_answer) in a
        .corevalues
        .response_and_weights
        .iter()
        .zip(b.corevalues.response_and_weights.iter())
    {
        let diff = f32::abs(a_answer.response.normalized() - b_answer.response.normalized());
        let similarity = 1.0 - diff;
        let weight = subject_chosen_weight_scale_factor
            * a_answer.weight.normalized()
            * CORE_VALUES_SECTION_WEIGHT;
        total += similarity * weight;
        weight_sum += weight;
    }

    (total, weight_sum)
}

fn process_relationship_dynamics(
    a: &QuestionnaireResponse,
    b: &QuestionnaireResponse,
    subject_chosen_weight_scale_factor: f32,
) -> (f32, f32) {
    const RELATIONSHIP_DYNAMICS_SECTION_WEIGHT: f32 = 1.0;
    let mut total = 0.0;
    let mut weight_sum = 0.0;
    for (a_answer, b_answer) in a
        .relationshipdynamics
        .response_and_weights
        .iter()
        .zip(b.relationshipdynamics.response_and_weights.iter())
    {
        let diff = f32::abs(a_answer.response.normalized() - b_answer.response.normalized());
        let similarity = 1.0 - diff;
        let weight = subject_chosen_weight_scale_factor
            * a_answer.weight.normalized()
            * RELATIONSHIP_DYNAMICS_SECTION_WEIGHT;
        total += similarity * weight;
        weight_sum += weight;
    }

    for (a_answer, b_answer) in a
        .relationshipdynamics
        .responses
        .iter()
        .zip(b.relationshipdynamics.responses.iter())
    {
        let diff = f32::abs(a_answer.normalized() - b_answer.normalized());
        let similarity = 1.0 - diff;
        let weight = RELATIONSHIP_DYNAMICS_SECTION_WEIGHT;
        total += similarity * weight;
        weight_sum += weight;
    }

    (total, weight_sum)
}

fn process_lifestyle_money(a: &QuestionnaireResponse, b: &QuestionnaireResponse) -> (f32, f32) {
    const LIFESTYLE_MONEY_SECTION_WEIGHT: f32 = 0.8;
    const NUM_CHILDREN_QUESTION_WEIGHT: f32 = 0.75;

    let mut total = 0.0;
    let mut weight_sum = 0.0;
    for (a_answer, b_answer) in a
        .lifestylemoney
        .responses
        .iter()
        .zip(b.lifestylemoney.responses.iter())
    {
        let diff = f32::abs(a_answer.normalized() - b_answer.normalized());
        let similarity = 1.0 - diff;
        let weight = LIFESTYLE_MONEY_SECTION_WEIGHT;
        total += similarity * weight;
        weight_sum += weight;
    }

    let diff = f32::abs(
        a.lifestylemoney.num_children.normalized() - b.lifestylemoney.num_children.normalized(),
    );
    let similarity = 1.0 - diff;
    let weight = LIFESTYLE_MONEY_SECTION_WEIGHT * NUM_CHILDREN_QUESTION_WEIGHT;
    total += similarity * weight;
    weight_sum += weight;

    (total, weight_sum)
}

fn process_social_style(a: &QuestionnaireResponse, b: &QuestionnaireResponse) -> (f32, f32) {
    const SOCIAL_STYLE_SECTION_WEIGHT: f32 = 0.8;

    let mut total = 0.0;
    let mut weight_sum = 0.0;
    for (a_answer, b_answer) in a
        .socialstyle
        .responses
        .iter()
        .zip(b.socialstyle.responses.iter())
    {
        let diff = f32::abs(a_answer.normalized() - b_answer.normalized());
        let similarity = 1.0 - diff;
        let weight = SOCIAL_STYLE_SECTION_WEIGHT;
        total += similarity * weight;
        weight_sum += weight;
    }

    (total, weight_sum)
}

fn process_interests(a: &QuestionnaireResponse, b: &QuestionnaireResponse) -> (f32, f32) {
    // Half weight, because similar interests don't predict long-term compatibility
    const INTERESTS_SECTION_WEIGHT: f32 = 0.5;

    let mut total = 0.0;
    let mut weight_sum = 0.0;
    for (a_answer, b_answer) in a
        .interests
        .responses
        .iter()
        .zip(b.interests.responses.iter())
    {
        let diff = f32::abs(a_answer.normalized() - b_answer.normalized());
        let similarity = 1.0 - diff;
        let weight = INTERESTS_SECTION_WEIGHT;
        total += similarity * weight;
        weight_sum += weight;
    }

    (total, weight_sum)
}

fn process_age(a: &QuestionnaireResponse, b: &QuestionnaireResponse) -> (f32, f32) {
    // Similar ages are important.
    const AGE_QUESTION_WEIGHT: f32 = 4.2;

    let diff = a.demographics.age.0.abs_diff(b.demographics.age.0);
    // Divide by 1 + the max spread to keep similarity > 0
    let similarity = 1.0 - (f32::from(diff) / f32::from(Age::MAX_AGE - Age::MIN_AGE + 1));
    let weight = AGE_QUESTION_WEIGHT;

    (similarity * weight, weight)
}

/// Which of the 8 cross-matched pairs use a bipolar preference scale, where the
/// respondent picks a point on the trait axis, so `1 - |want - have|` is the right
/// similarity measure. The rest use an importance scale ("...in a partner matters",
/// 1 = not at all), where wanting less of a trait must not penalize a partner who has
/// more of it — see `crossmatch_similarity`. Indices line up with
/// `PartnerPreferences::crossmatched` / `SelfDescription::crossmatched`
/// (parsing.rs's `crossmatched_indices` arrays).
const CROSSMATCH_IS_BIPOLAR: [bool; 8] = [
    true,  // 0 plans carefully            <-> prefers a planner vs. go-with-the-flow
    false, // 1 artistic side               <-> artistic side matters
    true,  // 2 energetic and outgoing      <-> prefers homebody vs. very social
    false, // 3 goal-oriented               <-> ambition matters
    false, // 4 dry sense of humor          <-> dry humor matters
    false, // 5 enjoys intellectual debate  <-> intellectually curious matters
    false, // 6 diet and nutrition          <-> health-conscious matters
    false, // 7 staying active              <-> active or fit matters
];

/// Similarity for one cross-matched pair: `want` is how much of the trait the subject
/// asked for in a partner; `have` is how much of the trait the candidate actually has.
/// Bipolar items compare position on the axis both ways. Importance items only
/// penalize a shortfall: wanting less of a trait than a partner has is never a
/// mismatch, since indifference ("doesn't matter") is satisfied by any amount.
fn crossmatch_similarity(is_bipolar: bool, want: f32, have: f32) -> f32 {
    if is_bipolar {
        1.0 - f32::abs(want - have)
    } else {
        1.0 - f32::max(0.0, want - have)
    }
}

fn process_self_and_partner(a: &QuestionnaireResponse, b: &QuestionnaireResponse) -> (f32, f32) {
    // Reduced because 8 of the 15 self-description items also feed the partner-preferences cross-match below. They score twice.
    const SELF_DESCRIPTION_SECTION_WEIGHT: f32 = 0.6;
    const PARTNER_PREFERENCES_SECTION_WEIGHT: f32 = 1.0;

    let mut total = 0.0;
    let mut weight_sum = 0.0;
    for (a_answer, b_answer) in a
        .selfdescription
        .direct
        .iter()
        .zip(b.selfdescription.direct.iter())
    {
        let diff = f32::abs(a_answer.normalized() - b_answer.normalized());
        let similarity = 1.0 - diff;
        let weight = SELF_DESCRIPTION_SECTION_WEIGHT;
        total += similarity * weight;
        weight_sum += weight;
    }

    for (a_answer, b_answer) in a
        .partnerpreferences
        .direct
        .iter()
        .zip(b.partnerpreferences.direct.iter())
    {
        let diff = f32::abs(a_answer.normalized() - b_answer.normalized());
        let similarity = 1.0 - diff;
        let weight = PARTNER_PREFERENCES_SECTION_WEIGHT;
        total += similarity * weight;
        weight_sum += weight;
    }

    // Calculate cross-matches
    for (k, (a_answer, b_answer)) in a
        .partnerpreferences
        .crossmatched
        .iter()
        .zip(b.selfdescription.crossmatched.iter())
        .enumerate()
    {
        let want = a_answer.normalized();
        let have = b_answer.normalized();
        let similarity = crossmatch_similarity(CROSSMATCH_IS_BIPOLAR[k], want, have);
        let weight = PARTNER_PREFERENCES_SECTION_WEIGHT;
        total += similarity * weight;
        weight_sum += weight;
    }

    (total, weight_sum)
}

/// Calculate how well b satisfies a's preferences. Not symmetric. Only used by tests;
/// `build_scored_pairs` calls `directional_score_with_scale` directly so it can compute
/// each person's scale factor once instead of on every call.
#[cfg(test)]
fn directional_score(a: &QuestionnaireResponse, b: &QuestionnaireResponse) -> f32 {
    let subject_chosen_weight_scale_factor = calculate_subject_chosen_weight_scale_factor(a);
    directional_score_with_scale(a, b, subject_chosen_weight_scale_factor)
}

/// Same as `directional_score`, but takes `a`'s weight-scale factor instead of
/// recomputing it. The factor depends only on `a`, so a caller scoring `a` against many
/// candidates (`build_scored_pairs`) can compute it once and reuse it here, rather than
/// recomputing it for every candidate.
fn directional_score_with_scale(
    a: &QuestionnaireResponse,
    b: &QuestionnaireResponse,
    subject_chosen_weight_scale_factor: f32,
) -> f32 {
    let mut total = 0.0;
    let mut weight_sum = 0.0;

    let core_value_results = process_core_values(a, b, subject_chosen_weight_scale_factor);
    total += core_value_results.0;
    weight_sum += core_value_results.1;

    let relationship_dynamics_results =
        process_relationship_dynamics(a, b, subject_chosen_weight_scale_factor);
    total += relationship_dynamics_results.0;
    weight_sum += relationship_dynamics_results.1;

    let lifestyle_money_results = process_lifestyle_money(a, b);
    total += lifestyle_money_results.0;
    weight_sum += lifestyle_money_results.1;

    let social_style_results = process_social_style(a, b);
    total += social_style_results.0;
    weight_sum += social_style_results.1;

    let interests_results = process_interests(a, b);
    total += interests_results.0;
    weight_sum += interests_results.1;

    let self_and_partner_results = process_self_and_partner(a, b);
    total += self_and_partner_results.0;
    weight_sum += self_and_partner_results.1;

    let age_result = process_age(a, b);
    total += age_result.0;
    weight_sum += age_result.1;

    total / weight_sum
}

/// Combine two directional scores for the same pair into one, so that a's compatibility
/// with b and b's compatibility with a are both contained in a single score. Symmetric:
/// `combine(ab, ba) == combine(ba, ab)`.
fn combine(ab: f32, ba: f32) -> f32 {
    // How satisfied is the least satisfied person.
    let min_score = ab.min(ba);

    // The overall happiness of the pair.
    let average = f32::midpoint(ab, ba);

    // Lean more toward the least satisfied but break ties with average.
    (0.8 * min_score) + (0.2 * average)
}

/// Calculated the mutual score of a and b in a non-directional way, so that a's compatibility
/// with b and b's compatibility with a are both contained in a single score. Only used by
/// tests, as the reference formula `build_scored_pairs`'s calibrated scoring is built on.
#[cfg(test)]
fn mutual_score(a: &QuestionnaireResponse, b: &QuestionnaireResponse) -> f32 {
    combine(directional_score(a, b), directional_score(b, a))
}

/// The two numbers a scored pair carries. Kept separate because they answer different
/// questions: `display` is the true mutual compatibility shown to participants and used
/// in the QUALITY diagnostics; `rank` is only used to order candidates during shortlist
/// assignment (see `build_scored_pairs` for why they can differ).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PairScore {
    pub display: f32,
    pub rank: f32,
}

/// Builds a hashmap of (male.id, female.id) -> score. No pairing will be in the map if
/// there are dealbreakers on either side. Score takes into account how compatible each
/// side is with the other, so no need for a reverse map (female.id, male.id) -> score.
fn build_scored_pairs(
    responses: &[QuestionnaireResponse],
    collect_stats: bool,
) -> (FxHashMap<(&str, &str), PairScore>, Option<PairsStats>) {
    // Empirically good enough for 1000 people. Technically the capacity should be (responses / 2) ^ 2 assuming
    // equal numbers of men and women. But dealbreakers shrink that down.
    let mut pairs = FxHashMap::with_capacity_and_hasher(50000, FxBuildHasher);

    let males: Vec<_> = responses
        .iter()
        .filter(|r| r.demographics.gender == Gender::Male)
        .collect();
    let females: Vec<_> = responses
        .iter()
        .filter(|r| r.demographics.gender == Gender::Female)
        .collect();

    let mut stats = if collect_stats {
        Some(PairsStats {
            male_count: males.len(),
            female_count: females.len(),
            dealbreaker_eliminated: 0,
            dealbreaker_by_wants_children: 0,
            dealbreaker_by_stay_local: 0,
            dealbreaker_by_marriage_timeline: 0,
            dealbreaker_by_religion: 0,
        })
    } else {
        None
    };

    // Each person's weight-scale factor depends only on them, not on any candidate, so
    // it's computed once per person here instead of twice per pair inside the loop below.
    let male_scale: FxHashMap<&str, f32> = males
        .iter()
        .map(|m| (m.id(), calculate_subject_chosen_weight_scale_factor(m)))
        .collect();
    let female_scale: FxHashMap<&str, f32> = females
        .iter()
        .map(|f| (f.id(), calculate_subject_chosen_weight_scale_factor(f)))
        .collect();

    // First pass: score every surviving pair in both directions, and accumulate each
    // person's own outgoing-score total (their directional score toward every candidate
    // they're compatible with) so it can be mean-centered in the second pass.
    let mut raw: Vec<(&str, &str, f32, f32)> = Vec::new();
    let mut outgoing_sum: FxHashMap<&str, f32> = FxHashMap::default();
    let mut outgoing_count: FxHashMap<&str, usize> = FxHashMap::default();

    for male in &males {
        let m_scale = male_scale[male.id()];
        for female in &females {
            if let Err(cause) = passes_dealbreakers(male, female) {
                if let Some(ref mut s) = stats {
                    s.dealbreaker_eliminated += 1;
                    match cause {
                        DealbreakerCause::WantsChildren => s.dealbreaker_by_wants_children += 1,
                        DealbreakerCause::StayLocal => s.dealbreaker_by_stay_local += 1,
                        DealbreakerCause::MarriageTimeline => {
                            s.dealbreaker_by_marriage_timeline += 1;
                        }
                        DealbreakerCause::Religion => s.dealbreaker_by_religion += 1,
                    }
                }
                continue;
            }

            let f_scale = female_scale[female.id()];
            // How well female satisfies male, and vice versa.
            let male_wants = directional_score_with_scale(male, female, m_scale);
            let female_wants = directional_score_with_scale(female, male, f_scale);

            *outgoing_sum.entry(male.id()).or_insert(0.0) += male_wants;
            *outgoing_count.entry(male.id()).or_insert(0) += 1;
            *outgoing_sum.entry(female.id()).or_insert(0.0) += female_wants;
            *outgoing_count.entry(female.id()).or_insert(0) += 1;

            raw.push((male.id(), female.id(), male_wants, female_wants));
        }
    }

    // A person's own mean directional score toward their candidates. Nobody rates anyone
    // directly here — this score comes entirely from comparing a person's own answers,
    // preferences, and self-assigned importance weights against each candidate's answers.
    // But that means some people's own answers structurally score higher or lower against
    // nearly everyone: narrow partner preferences score low against most of the pool,
    // answers near the pool's center score higher, independent of fit with any specific
    // candidate. Subtracting this baseline before combining keeps that structural effect
    // out of who ranks above whom, without touching the display score participants see.
    //
    // Why this can't lose real signal: subtracting a constant from both of a person's
    // directional scores cannot reorder their own candidate list — it only shifts which
    // side binds the min() inside `combine`. Before centering, someone with narrow
    // preferences scored low against everyone, so their side always bound the min, so
    // every pair containing them looked bad regardless of who the candidate was — they
    // got buried on every list. Centering makes that min compare "who's getting the
    // worse deal by their own standards" instead of "whose absolute number is smaller",
    // without overriding anyone's actual stated preferences.
    //
    // This only corrects a person's own outgoing tendency, not how much they're wanted by
    // others in return, so it blunts `person_effect_share` rather than eliminating it —
    // see `person_effect_share_calibrated` and `headroom_ranking` in diagnostics.rs for
    // how much it actually blunts, measured on real data. A person with fewer than 2
    // candidates has no meaningful average to subtract, so calibration is a no-op for
    // them.
    let mean_outgoing = |id: &str| match outgoing_count.get(id) {
        Some(&count) if count >= 2 => outgoing_sum[id] / count as f32,
        _ => 0.0,
    };

    for (male_id, female_id, male_wants, female_wants) in raw {
        let display = combine(male_wants, female_wants);
        let rank = combine(
            male_wants - mean_outgoing(male_id),
            female_wants - mean_outgoing(female_id),
        );
        pairs.insert((male_id, female_id), PairScore { display, rank });
    }

    (pairs, stats)
}

type Shortlists = FxHashMap<String, Vec<(String, f32)>>;

/// Per person, their candidates ranked descending by `PairScore::rank`. Each entry is
/// `(candidate_id, rank_score, display_score)`; `rank_score` decides ordering,
/// `display_score` is what gets shown once a candidate is picked.
pub(crate) type RankedCandidates<'a> = FxHashMap<&'a str, Vec<(&'a str, f32, f32)>>;

/// Precompute each person's ranked candidate list (descending by rank score). This is the
/// deterministic, expensive-ish part of matching (`O(pairs)`), kept separate from the
/// cheap randomized assignment in `assign_shortlists` so the diagnostics can also see it.
fn build_ranked_candidates<'a>(
    ids: &[&'a str],
    pairs: &'a FxHashMap<(&'a str, &'a str), PairScore>,
) -> RankedCandidates<'a> {
    let mut ranked_candidates: RankedCandidates<'a> = FxHashMap::default();

    // Estimated that we don't have dealbreakers with 1/2 of the other gender (1/2 of ids, assuming equal gender ratios).
    let estimated_rank_capacity = ids.len() / 4;
    for (&(a, b), score) in pairs {
        ranked_candidates
            .entry(a)
            .or_insert(Vec::with_capacity(estimated_rank_capacity))
            .push((b, score.rank, score.display));
        ranked_candidates
            .entry(b)
            .or_insert(Vec::with_capacity(estimated_rank_capacity))
            .push((a, score.rank, score.display));
    }

    // Ensure every id has an entry, even people who are not in pairs (only one of their gender, or dealbreakers that exclude everyone else)
    for pid in ids {
        ranked_candidates.entry(pid).or_default();
    }

    for value in ranked_candidates.values_mut() {
        // Reverse sort by rank score, so the best-still-available candidate comes first.
        value.sort_unstable_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .expect("Should be able to compare floats")
        });
    }

    ranked_candidates
}

/// Run one round of shortlist assignment: shuffle the incomplete set, then offer each person
/// their next-best available candidate under the current cap. Returns true if at least one
/// pick was made (progress), false if the round was completely blocked.
#[allow(clippy::too_many_arguments)]
fn run_round<'a>(
    incomplete: &mut FxHashSet<&&'a str>,
    rng: &mut StdRng,
    target_shortlist: usize,
    cap: usize,
    ranked_candidates: &'a RankedCandidates<'a>,
    shortlists: &mut FxHashMap<String, Vec<(String, f32)>>,
    appearance_count: &mut FxHashMap<&'a str, usize>,
    excess_ranks: &mut Vec<usize>,
) -> bool {
    let mut order: Vec<_> = incomplete.iter().copied().collect();
    order.shuffle(rng);

    let mut made_progress = false;
    for pid in order {
        if shortlists
            .get(*pid)
            .is_some_and(|sl| sl.len() >= target_shortlist)
        {
            incomplete.remove(&pid);
            continue;
        }

        if let Some((other_id, score, rank)) =
            next_available(pid, cap, ranked_candidates, shortlists, appearance_count)
        {
            let sl = shortlists
                .entry(pid.to_string())
                .or_insert(Vec::with_capacity(target_shortlist));
            let position = sl.len() + 1;
            sl.push((other_id.to_string(), score));
            *appearance_count.entry(other_id).or_default() += 1;
            excess_ranks.push((rank + 1).saturating_sub(position));
            made_progress = true;
        }
    }

    made_progress
}

fn assign_shortlists<'a>(
    ids: &[&'a str],
    ranked_candidates: &'a RankedCandidates<'a>,
    rng: &mut StdRng,
    collect_stats: bool,
    target_shortlist: usize,
    max_appearances: usize,
    max_appearances_relaxed: usize,
) -> (Shortlists, Option<ShortlistStats>) {
    let mut cap = max_appearances;
    let mut cap_relaxed = false;
    let mut appearance_count: FxHashMap<&str, usize> = FxHashMap::default();
    let mut shortlists: FxHashMap<String, Vec<(String, f32)>> = FxHashMap::default();
    let mut excess_ranks: Vec<usize> = Vec::new();

    for pid in ids {
        shortlists.entry(pid.to_string()).or_default();
    }

    let mut incomplete: FxHashSet<_> = ids.iter().collect();

    // Outer loop: raise the appearance cap by 1 each time we stall, up to max_appearances_relaxed.
    // Always aim for target_shortlist — the cap grows only as needed to keep making progress.
    'outer: loop {
        // Inner loop: run rounds at the current cap until a full pass makes no progress.
        loop {
            let made_progress = run_round(
                &mut incomplete,
                rng,
                target_shortlist,
                cap,
                ranked_candidates,
                &mut shortlists,
                &mut appearance_count,
                &mut excess_ranks,
            );
            if !made_progress {
                break;
            }
        }

        if incomplete.is_empty() {
            break 'outer;
        }

        // Stalled at this cap. Raise by 1 if possible; otherwise capacity is exhausted.
        if cap < max_appearances_relaxed {
            cap += 1;
            cap_relaxed = true;
        } else {
            break 'outer;
        }
    }

    let shortlist_stats = if collect_stats {
        let owned_appearance: FxHashMap<String, usize> = appearance_count
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect();
        Some(ShortlistStats {
            cap_relaxed,
            appearance_count: owned_appearance,
            served_excess_ranks: excess_ranks,
        })
    } else {
        None
    };

    (shortlists, shortlist_stats)
}

fn next_available<'a>(
    pid: &str,
    current_cap: usize,
    ranked_candidates: &'a RankedCandidates<'a>,
    shortlists: &FxHashMap<String, Vec<(String, f32)>>,
    appearance_count: &FxHashMap<&str, usize>,
) -> Option<(&'a str, f32, usize)> {
    for (rank, (other_id, _rank_score, display_score)) in ranked_candidates[pid].iter().enumerate()
    {
        if shortlists
            .get(pid)
            .is_some_and(|sl| sl.iter().any(|i| &i.0 == other_id))
        {
            continue;
        }
        if appearance_count.get(other_id).copied().unwrap_or(0) >= current_cap {
            continue;
        }
        return Some((other_id, *display_score, rank));
    }

    None
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // exact comparisons against deterministic, hand-computed values
mod tests {
    use crate::parsing::{Dealbreakers, Demographics, FourChoiceResponse, MyReligiousCommitment};
    use crate::rng_and_seed;

    use super::*;

    #[test]
    fn crossmatch_similarity_shortfall_only_for_importance_scale() {
        let normalized = |v: u8| FourChoiceResponse(v).normalized();
        // Importance scale (is_bipolar = false): indifference is always satisfied;
        // wanting more of a trait than a partner has costs the shortfall, but wanting
        // less than the partner has is never a mismatch.
        assert_eq!(
            crossmatch_similarity(false, normalized(1), normalized(4)),
            1.0
        ); // don't care, partner has plenty -> perfect
        assert_eq!(
            crossmatch_similarity(false, normalized(4), normalized(1)),
            0.0
        ); // essential, partner has none -> worst
        assert_eq!(
            crossmatch_similarity(false, normalized(3), normalized(4)),
            1.0
        ); // wanted less than the partner has -> no penalty
        assert!(
            (crossmatch_similarity(false, normalized(3), normalized(2)) - 2.0 / 3.0).abs() < 1e-6
        );
    }

    #[test]
    fn crossmatch_similarity_bipolar_penalizes_either_direction() {
        let normalized = |v: u8| FourChoiceResponse(v).normalized();
        // Bipolar scale (is_bipolar = true): distance in either direction costs the same.
        assert_eq!(
            crossmatch_similarity(true, normalized(1), normalized(4)),
            0.0
        );
        assert_eq!(
            crossmatch_similarity(true, normalized(4), normalized(1)),
            0.0
        );
        assert_eq!(
            crossmatch_similarity(true, normalized(1), normalized(1)),
            1.0
        );
    }

    #[test]
    fn calibration_is_noop_with_only_one_candidate() {
        let male = QuestionnaireResponse {
            demographics: Demographics {
                email: "male@example.com".to_string(),
                gender: Gender::Male,
                ..Default::default()
            },
            ..Default::default()
        };
        let female = QuestionnaireResponse {
            demographics: Demographics {
                email: "female@example.com".to_string(),
                gender: Gender::Female,
                age: Age(30),
                ..Default::default()
            },
            ..Default::default()
        };
        // With exactly one candidate each, neither side has a meaningful outgoing
        // average to subtract, so the calibrated rank score must equal the display score.
        let responses = [male, female];
        let (pairs, _) = build_scored_pairs(&responses, false);
        assert_eq!(pairs.len(), 1);
        let score = pairs.values().next().expect("exactly one pair");
        assert_eq!(score.rank, score.display);
    }

    #[test]
    fn calibration_can_change_rank_order_but_never_display() {
        use crate::parsing::FourChoiceResponse;

        let male = |email: &str| QuestionnaireResponse {
            demographics: Demographics {
                email: email.to_string(),
                gender: Gender::Male,
                age: Age(30),
                ..Default::default()
            },
            ..Default::default()
        };
        let female = |email: &str| QuestionnaireResponse {
            demographics: Demographics {
                email: email.to_string(),
                gender: Gender::Female,
                age: Age(30),
                ..Default::default()
            },
            ..Default::default()
        };

        let m1 = male("m1@example.com");
        let mut m2 = male("m2@example.com");
        let mut f_narrow = female("f_narrow@example.com");
        let f_typical = female("f_typical@example.com");

        // f_narrow wants three traits (non-bipolar "importance" crossmatch items)
        // strongly, but every male defaults to not having any of them. That shortfall
        // drags f_narrow's own directional score toward every male down uniformly — a
        // structural, answer-driven person effect, not a reflection of fit with any
        // specific candidate. (Non-bipolar, so indifference — the default "want" — is
        // always satisfied regardless of what a candidate "has"; only f_narrow's own
        // non-default wants create a shortfall here, and only on her own outgoing side.)
        f_narrow.partnerpreferences.crossmatched[1] = FourChoiceResponse(4);
        f_narrow.partnerpreferences.crossmatched[4] = FourChoiceResponse(4);
        f_narrow.partnerpreferences.crossmatched[5] = FourChoiceResponse(4);

        // m2 specifically wants a fourth, distinct non-bipolar trait that only f_narrow
        // has. This is genuine pair-specific compatibility, smaller in magnitude than
        // f_narrow's person effect above: m2 is a better match for f_narrow than for
        // f_typical, but not by enough to overcome her person effect on raw display.
        m2.partnerpreferences.crossmatched[3] = FourChoiceResponse(4);
        f_narrow.selfdescription.crossmatched[3] = FourChoiceResponse(4);

        let responses = [m1.clone(), m2.clone(), f_narrow.clone(), f_typical.clone()];
        let (pairs, _) = build_scored_pairs(&responses, false);
        assert_eq!(pairs.len(), 4);

        let get = |m: &QuestionnaireResponse, f: &QuestionnaireResponse| {
            *pairs.get(&(m.id(), f.id())).expect("pair should be scored")
        };

        // Regression guard: calibration must never change the display score. Check it
        // against an independently-computed value, not just against itself.
        for (m, f) in [
            (&m1, &f_narrow),
            (&m1, &f_typical),
            (&m2, &f_narrow),
            (&m2, &f_typical),
        ] {
            let score = get(m, f);
            let independent_display = mutual_score(m, f);
            assert!(
                (score.display - independent_display).abs() < 1e-6,
                "display score for ({}, {}) should match an independent recomputation: {} vs {}",
                m.id(),
                f.id(),
                score.display,
                independent_display
            );
        }

        let m2_narrow = get(&m2, &f_narrow);
        let m2_typical = get(&m2, &f_typical);

        // The construction's whole point: f_narrow's person effect (from the shortfall
        // above) is enough to make her look worse than f_typical by raw display score,
        // even though she is genuinely the better match for m2 specifically.
        assert!(
            m2_narrow.display < m2_typical.display,
            "expected f_narrow's person effect to drag her display score below f_typical's for m2: {} vs {}",
            m2_narrow.display,
            m2_typical.display
        );
        // Calibration should recover the genuine pair-specific fit: once each woman's
        // own outgoing average is subtracted out, f_narrow should rank above f_typical
        // for m2 — the opposite order from display, and never the case with only one
        // candidate per side (see `calibration_is_noop_with_only_one_candidate`).
        assert!(
            m2_narrow.rank > m2_typical.rank,
            "expected calibration to flip m2's ranking of f_narrow above f_typical: {} vs {}",
            m2_narrow.rank,
            m2_typical.rank
        );
    }

    #[test]
    fn children_dealbreaker_opposite() {
        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                wants_children: YesNoMaybeResponse::No,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                wants_children: YesNoMaybeResponse::Yes,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(passes_dealbreakers(&a, &b).is_err());
        assert!(passes_dealbreakers(&b, &a).is_err());
    }

    #[test]
    fn children_dealbreaker_same() {
        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                wants_children: YesNoMaybeResponse::No,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                wants_children: YesNoMaybeResponse::Yes,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(passes_dealbreakers(&a, &a).is_ok());
        assert!(passes_dealbreakers(&b, &b).is_ok());
    }

    #[test]
    fn children_dealbreaker_maybe() {
        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                wants_children: YesNoMaybeResponse::No,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                wants_children: YesNoMaybeResponse::Yes,
                ..Default::default()
            },
            ..Default::default()
        };

        let c = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                wants_children: YesNoMaybeResponse::Maybe,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(passes_dealbreakers(&a, &c).is_ok());
        assert!(passes_dealbreakers(&c, &a).is_ok());
        assert!(passes_dealbreakers(&b, &c).is_ok());
        assert!(passes_dealbreakers(&c, &b).is_ok());
        assert!(passes_dealbreakers(&c, &c).is_ok());
    }

    #[test]
    fn stay_local_dealbreaker_opposite() {
        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                stay_local: YesNoMaybeResponse::No,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                stay_local: YesNoMaybeResponse::Yes,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(passes_dealbreakers(&a, &b).is_err());
        assert!(passes_dealbreakers(&b, &a).is_err());
    }

    #[test]
    fn stay_local_dealbreaker_same() {
        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                stay_local: YesNoMaybeResponse::No,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                stay_local: YesNoMaybeResponse::Yes,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(passes_dealbreakers(&a, &a).is_ok());
        assert!(passes_dealbreakers(&b, &b).is_ok());
    }

    #[test]
    fn stay_local_dealbreaker_maybe() {
        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                stay_local: YesNoMaybeResponse::No,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                stay_local: YesNoMaybeResponse::Yes,
                ..Default::default()
            },
            ..Default::default()
        };

        let c = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                stay_local: YesNoMaybeResponse::Maybe,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(passes_dealbreakers(&a, &c).is_ok());
        assert!(passes_dealbreakers(&c, &a).is_ok());
        assert!(passes_dealbreakers(&b, &c).is_ok());
        assert!(passes_dealbreakers(&c, &b).is_ok());
        assert!(passes_dealbreakers(&c, &c).is_ok());
    }

    #[test]
    fn timeline_dealbreaker_nonadjacent() {
        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                marriage_timeline: MarriageTimelineResponse::ZeroToTwo,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                marriage_timeline: MarriageTimelineResponse::FivePlus,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(passes_dealbreakers(&a, &b).is_err());
        assert!(passes_dealbreakers(&b, &a).is_err());
    }

    #[test]
    fn timeline_dealbreaker_same() {
        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                marriage_timeline: MarriageTimelineResponse::FivePlus,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                marriage_timeline: MarriageTimelineResponse::ZeroToTwo,
                ..Default::default()
            },
            ..Default::default()
        };

        let c = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                marriage_timeline: MarriageTimelineResponse::TwoToFive,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(passes_dealbreakers(&a, &a).is_ok());
        assert!(passes_dealbreakers(&b, &b).is_ok());
        assert!(passes_dealbreakers(&c, &c).is_ok());
    }

    #[test]
    fn timeline_dealbreaker_adjacent() {
        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                marriage_timeline: MarriageTimelineResponse::ZeroToTwo,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                marriage_timeline: MarriageTimelineResponse::TwoToFive,
                ..Default::default()
            },
            ..Default::default()
        };

        let c = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                marriage_timeline: MarriageTimelineResponse::FivePlus,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(passes_dealbreakers(&a, &b).is_ok());
        assert!(passes_dealbreakers(&b, &c).is_ok());
        assert!(passes_dealbreakers(&c, &b).is_ok());
        assert!(passes_dealbreakers(&b, &a).is_ok());
    }

    #[test]
    fn religous_dealbreaker_no_pref() {
        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                my_religious_commitment: MyReligiousCommitment::new("1").unwrap(),
                partners_religious_commitment: PartnersReligionResponse::DoesNotMatter,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                my_religious_commitment: MyReligiousCommitment::new("5").unwrap(),
                partners_religious_commitment: PartnersReligionResponse::DoesNotMatter,
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(passes_dealbreakers(&a, &b).is_ok());
        assert!(passes_dealbreakers(&b, &a).is_ok());
    }

    #[test]
    fn religous_dealbreaker_satisfied() {
        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                my_religious_commitment: MyReligiousCommitment::new("1").unwrap(),
                partners_religious_commitment: PartnersReligionResponse::Within1Level,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                my_religious_commitment: MyReligiousCommitment::new("2").unwrap(),
                partners_religious_commitment: PartnersReligionResponse::DoesNotMatter,
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(passes_dealbreakers(&a, &b).is_ok());
        assert!(passes_dealbreakers(&b, &a).is_ok());

        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                my_religious_commitment: MyReligiousCommitment::new("3").unwrap(),
                partners_religious_commitment: PartnersReligionResponse::DoesNotMatter,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                my_religious_commitment: MyReligiousCommitment::new("2").unwrap(),
                partners_religious_commitment: PartnersReligionResponse::Within1Level,
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(passes_dealbreakers(&a, &b).is_ok());
        assert!(passes_dealbreakers(&b, &a).is_ok());

        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                my_religious_commitment: MyReligiousCommitment::new("3").unwrap(),
                partners_religious_commitment: PartnersReligionResponse::Same,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                my_religious_commitment: MyReligiousCommitment::new("3").unwrap(),
                partners_religious_commitment: PartnersReligionResponse::Same,
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(passes_dealbreakers(&a, &b).is_ok());
        assert!(passes_dealbreakers(&b, &a).is_ok());
    }

    #[test]
    fn religous_dealbreaker_unsatisfied() {
        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                my_religious_commitment: MyReligiousCommitment::new("1").unwrap(),
                partners_religious_commitment: PartnersReligionResponse::Within1Level,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                my_religious_commitment: MyReligiousCommitment::new("3").unwrap(),
                partners_religious_commitment: PartnersReligionResponse::DoesNotMatter,
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(passes_dealbreakers(&a, &b).is_err());
        assert!(passes_dealbreakers(&b, &a).is_err());

        let a = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                my_religious_commitment: MyReligiousCommitment::new("1").unwrap(),
                partners_religious_commitment: PartnersReligionResponse::Same,
                ..Default::default()
            },
            ..Default::default()
        };

        let b = QuestionnaireResponse {
            dealbreakers: Dealbreakers {
                my_religious_commitment: MyReligiousCommitment::new("2").unwrap(),
                partners_religious_commitment: PartnersReligionResponse::DoesNotMatter,
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(passes_dealbreakers(&a, &b).is_err());
        assert!(passes_dealbreakers(&b, &a).is_err());
    }

    #[test]
    fn test_default_mutual_score() {
        let default = QuestionnaireResponse::default();
        let mutual_score = mutual_score(&default, &default);
        assert_eq!(mutual_score, 1.0);
    }

    #[test]
    fn test_empty_create_matches() {
        let (mut rng, seed) = rng_and_seed(None);
        let (matches, _) =
            create_matches(&[], &mut rng, seed, true, false, 5, 12, 14, None).unwrap();
        assert_eq!(matches, Matches(vec![],));
    }

    #[test]
    fn test_one_item_create_matches() {
        let (mut rng, seed) = rng_and_seed(None);
        let (matches, _) = create_matches(
            &[QuestionnaireResponse::default()],
            &mut rng,
            seed,
            true,
            false,
            5,
            12,
            14,
            None,
        )
        .unwrap();
        assert_eq!(matches.0.len(), 1);
        assert!(matches.0[0].shortlist.is_empty());
        assert_eq!(
            matches,
            Matches(vec![MatchCard {
                name: String::new(),
                email: "example@example.com".to_string(),
                shortlist: vec![],
            }],)
        );
    }

    #[test]
    fn test_two_items_create_matches() {
        let first = QuestionnaireResponse {
            demographics: Demographics {
                name: "Candidate A".to_string(),
                age: Age(34),
                email: "first".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let second = QuestionnaireResponse {
            demographics: Demographics {
                name: "Candidate B".to_string(),
                age: Age(26),
                email: "second".to_string(),
                gender: Gender::Female,
            },
            ..Default::default()
        };
        let (mut rng, seed) = rng_and_seed(None);
        let (matches, _) = create_matches(
            &[second, first],
            &mut rng,
            seed,
            true,
            false,
            5,
            12,
            14,
            None,
        )
        .unwrap();
        assert_eq!(
            matches,
            Matches(vec![
                MatchCard {
                    name: "Candidate A".to_string(),
                    email: "first".to_string(),
                    shortlist: vec![ShortlistMatch {
                        name: "Candidate B".to_string(),
                        age: Age(26),
                        email: "second".to_string(),
                        freeresponse: FreeResponse { responses: vec![] },
                        score: 0.942_973_55
                    }]
                },
                MatchCard {
                    name: "Candidate B".to_string(),
                    email: "second".to_string(),
                    shortlist: vec![ShortlistMatch {
                        name: "Candidate A".to_string(),
                        age: Age(34),
                        email: "first".to_string(),
                        freeresponse: FreeResponse { responses: vec![] },
                        score: 0.942_973_55
                    }]
                }
            ],)
        );
    }

    #[test]
    fn test_invalid_debug_print_candidate_list() {
        let (mut rng, seed) = rng_and_seed(None);
        let result = create_matches(
            &[QuestionnaireResponse::default()],
            &mut rng,
            seed,
            true,
            false,
            5,
            12,
            14,
            Some("abc".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_non_existent_debug_print_candidate_list() {
        let (mut rng, seed) = rng_and_seed(None);
        let result = create_matches(
            &[QuestionnaireResponse::default()],
            &mut rng,
            seed,
            true,
            false,
            5,
            12,
            14,
            Some("ephraimkunz@example.com".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn text_good_debug_print_candidate_list() {
        let (mut rng, seed) = rng_and_seed(None);
        let result = create_matches(
            &[QuestionnaireResponse::default()],
            &mut rng,
            seed,
            true,
            false,
            5,
            12,
            14,
            Some("example@example.com".to_string()),
        );
        assert!(result.is_ok());
    }
}

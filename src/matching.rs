use std::fmt::{Display, Formatter};

use itertools::Itertools;
use rand::prelude::*;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use serde::Serialize;

use crate::parsing::{
    Age, FreeResponse, Gender, MarriageTimelineResponse, PartnersReligionResponse,
    QuestionnaireResponse, YesNoMaybeResponse,
};

use crate::diagnostics::{
    DealbreakerCause, Diagnostics, PairsStats, ShortlistStats, build_diagnostics,
};

use anyhow::Result;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Matches {
    pub cards: Vec<MatchCard>,
    pub print_scores: bool,
}

impl Display for Matches {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for card in &self.cards {
            writeln!(f, "{} ({})\n\nMatches:", card.name, card.email)?;
            for (index, m) in card.shortlist.iter().enumerate() {
                if self.print_scores {
                    writeln!(f, "\t{} - {} ({}) ({})", m.name, m.age.0, m.email, m.score)?;
                } else {
                    writeln!(f, "\t{} - {} ({})", m.name, m.age.0, m.email)?;
                }

                for (k, v) in &m.freeresponse.responses {
                    writeln!(f, "\t{k} {v}")?;
                }

                if index < (card.shortlist.len() - 1) {
                    writeln!(f)?;
                }
            }

            writeln!(
                f,
                "\n========================================================================\n"
            )?;
        }

        Ok(())
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

#[allow(clippy::too_many_arguments)]
pub fn create_matches(
    responses: &[QuestionnaireResponse],
    rng: &mut StdRng,
    rng_seed: u64,
    sort_shortlists_by_score: bool,
    print_scores: bool,
    collect_diagnostics: bool,
    target_shortlist: usize,
    max_appearances: usize,
    max_appearances_relaxed: usize,
    debug_print_candidate_list: Option<String>,
) -> (Matches, Option<Diagnostics>) {
    // Score all pairs
    let (pairs, pairs_stats) = build_scored_pairs(responses, collect_diagnostics);

    // Assign shortlists via round-robin
    let ids = responses
        .iter()
        .map(QuestionnaireResponse::id)
        .collect_vec();
    let (shortlists, shortlist_stats) = assign_shortlists(
        &ids,
        &pairs,
        rng,
        collect_diagnostics,
        target_shortlist,
        max_appearances,
        max_appearances_relaxed,
        debug_print_candidate_list,
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

    let result = Matches {
        cards: matches,
        print_scores,
    };

    let diagnostics = build_diagnostics(
        pairs_stats,
        shortlist_stats,
        &ids,
        &pairs,
        &result,
        target_shortlist,
        rng_seed,
    );

    (result, diagnostics)
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
    // Similar ages are important, but not the most important.
    const AGE_QUESTION_WEIGHT: f32 = 0.7;

    let diff = a.demographics.age.0.abs_diff(b.demographics.age.0);
    // Divide by 1 + the max spread to keep similarity > 0
    let similarity = 1.0 - (f32::from(diff) / f32::from(Age::MAX_AGE - Age::MIN_AGE + 1));
    let weight = AGE_QUESTION_WEIGHT;

    (similarity * weight, weight)
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
    for (a_answer, b_answer) in a
        .partnerpreferences
        .crossmatched
        .iter()
        .zip(b.selfdescription.crossmatched.iter())
    {
        let diff = f32::abs(a_answer.normalized() - b_answer.normalized());
        let similarity = 1.0 - diff;
        let weight = PARTNER_PREFERENCES_SECTION_WEIGHT;
        total += similarity * weight;
        weight_sum += weight;
    }

    (total, weight_sum)
}

/// Calculate how well b satisfies a's preferences. Not symmetric.
fn directional_score(a: &QuestionnaireResponse, b: &QuestionnaireResponse) -> f32 {
    let mut total = 0.0;
    let mut weight_sum = 0.0;

    let subject_chosen_weight_scale_factor = calculate_subject_chosen_weight_scale_factor(a);

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

/// Calculated the mutual score of a and b in a non-directional way, so that a's compatibility
/// with b and b's compatibility with a are both contained in a single score.
fn mutual_score(a: &QuestionnaireResponse, b: &QuestionnaireResponse) -> f32 {
    let ab = directional_score(a, b);
    let ba = directional_score(b, a);

    // How satisfied is the least satisfied person.
    let min_score = ab.min(ba);

    // The overall happiness of the pair.
    let average = f32::midpoint(ab, ba);

    // Lean more toward the least satisfied but break ties with average.
    (0.8 * min_score) + (0.2 * average)
}

/// Builds a hashmap of (male.id, female.id) -> score. No pairing will be in the map if
/// there are dealbreakers on either side. Score takes into account how compatible each
/// side is with the other, so no need for a reverse map (female.id, male.id) -> score.
fn build_scored_pairs(
    responses: &[QuestionnaireResponse],
    collect_stats: bool,
) -> (FxHashMap<(&str, &str), f32>, Option<PairsStats>) {
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

    for male in &males {
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

            let score = mutual_score(male, female);
            pairs.insert((male.id(), female.id()), score);
        }
    }

    (pairs, stats)
}

type Shortlists = FxHashMap<String, Vec<(String, f32)>>;

/// Run one round of shortlist assignment: shuffle the incomplete set, then offer each person
/// their next-best available candidate under the current cap. Returns true if at least one
/// pick was made (progress), false if the round was completely blocked.
#[allow(clippy::too_many_arguments)]
fn run_round<'a>(
    incomplete: &mut FxHashSet<&&'a str>,
    rng: &mut StdRng,
    target_shortlist: usize,
    cap: usize,
    ranked_candidates: &'a FxHashMap<&'a str, Vec<(&'a str, f32)>>,
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

#[allow(clippy::too_many_arguments)]
fn assign_shortlists(
    ids: &[&str],
    pairs: &FxHashMap<(&str, &str), f32>,
    rng: &mut StdRng,
    collect_stats: bool,
    target_shortlist: usize,
    max_appearances: usize,
    max_appearances_relaxed: usize,
    debug_print_candidate_list: Option<String>,
) -> (Shortlists, Option<ShortlistStats>) {
    let mut cap = max_appearances;
    let mut cap_relaxed = false;
    let mut appearance_count: FxHashMap<&str, usize> = FxHashMap::default();
    let mut shortlists: FxHashMap<String, Vec<(String, f32)>> = FxHashMap::default();
    let mut excess_ranks: Vec<usize> = Vec::new();

    // Precompute each person's ranked candidate list (descending score)
    let mut ranked_candidates: FxHashMap<&str, Vec<(&str, f32)>> = FxHashMap::default();

    // Estimated that we don't have dealbreakers with 1/2 of the other gender (1/2 of ids, assuming equal gender ratios).
    let estimated_rank_capacity = ids.len() / 4;
    for ((a, b), &s) in pairs {
        ranked_candidates
            .entry(a)
            .or_insert(Vec::with_capacity(estimated_rank_capacity))
            .push((b, s));
        ranked_candidates
            .entry(b)
            .or_insert(Vec::with_capacity(estimated_rank_capacity))
            .push((a, s));
    }

    // Ensure every id has an entry, even people who are not in pairs (only one of their gender, or dealbreakers that exclude everyone else)
    for pid in ids {
        ranked_candidates.entry(pid).or_default();
        shortlists.entry(pid.to_string()).or_default();
    }

    for value in ranked_candidates.values_mut() {
        // Reverse sort, so largest scores come first.
        value.sort_unstable_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .expect("Should be able to compare floats")
        });
    }

    if let Some(id) = debug_print_candidate_list {
        println!(
            "{}'s candidate_list{:?}",
            id,
            ranked_candidates[id.as_str()]
        );
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
                &ranked_candidates,
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
    ranked_candidates: &'a FxHashMap<&str, Vec<(&str, f32)>>,
    shortlists: &FxHashMap<String, Vec<(String, f32)>>,
    appearance_count: &FxHashMap<&str, usize>,
) -> Option<(&'a str, f32, usize)> {
    for (rank, (other_id, score)) in ranked_candidates[pid].iter().enumerate() {
        if shortlists
            .get(pid)
            .is_some_and(|sl| sl.iter().any(|i| &i.0 == other_id))
        {
            continue;
        }
        if appearance_count.get(other_id).copied().unwrap_or(0) >= current_cap {
            continue;
        }
        return Some((other_id, *score, rank));
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::parsing::{Dealbreakers, Demographics, MyReligiousCommitment};
    use crate::rng_and_seed;

    use super::*;

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
        assert_eq!(mutual_score, 1.0)
    }

    #[test]
    fn test_empty_create_matches() {
        let (mut rng, seed) = rng_and_seed(None);
        let (matches, _) = create_matches(&[], &mut rng, seed, true, true, false, 5, 12, 14, None);
        assert_eq!(
            matches,
            Matches {
                cards: vec![],
                print_scores: true
            }
        )
    }

    #[test]
    fn test_one_item_create_matches() {
        let (mut rng, seed) = rng_and_seed(None);
        let (matches, _) = create_matches(
            &[QuestionnaireResponse::default()],
            &mut rng,
            seed,
            true,
            true,
            false,
            5,
            12,
            14,
            None,
        );
        assert_eq!(matches.cards.len(), 1);
        assert!(matches.cards[0].shortlist.is_empty());
        assert_eq!(
            matches,
            Matches {
                cards: vec![MatchCard {
                    name: "".to_string(),
                    email: "".to_string(),
                    shortlist: vec![],
                }],
                print_scores: true
            }
        )
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
            true,
            false,
            5,
            12,
            14,
            None,
        );
        assert_eq!(
            matches,
            Matches {
                cards: vec![
                    MatchCard {
                        name: "Candidate A".to_string(),
                        email: "first".to_string(),
                        shortlist: vec![ShortlistMatch {
                            name: "Candidate B".to_string(),
                            age: Age(26),
                            email: "second".to_string(),
                            freeresponse: FreeResponse { responses: vec![] },
                            score: 0.98976606
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
                            score: 0.98976606
                        }]
                    }
                ],
                print_scores: true
            }
        )
    }
}

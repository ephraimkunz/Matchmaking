use std::fmt::{Display, Formatter};

use itertools::Itertools;
use rand::prelude::*;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};

use crate::parsing::{
    Age, FreeResponse, Gender, MarriageTimelineResponse, PartnersReligionResponse,
    QuestionnaireResponse, YesNoMaybeResponse,
};

use anyhow::Result;

#[derive(Debug, Default)]
pub struct Diagnostics {
    // Pool
    pub male_count: usize,
    pub female_count: usize,
    pub pairs_scored: usize,
    pub dealbreaker_eliminated: usize,
    pub dealbreaker_by_wants_children: usize,
    pub dealbreaker_by_stay_local: usize,
    pub dealbreaker_by_marriage_timeline: usize,
    pub dealbreaker_by_religion: usize,
    // Convergence
    pub cap_relaxed: bool,
    pub shortlist_full: usize,
    pub shortlist_acceptable: usize,
    pub shortlist_under_min: usize,
    pub shortlist_empty: usize,
    pub appearance_max: usize,
    pub appearance_stddev: f32,
    pub zero_appearance_participants: usize,
    // Quality
    pub histogram: [usize; 6], // <0.5, 0.5-0.6, 0.6-0.7, 0.7-0.8, 0.8-0.9, >=0.9
    pub regret_mean: f32,
    pub regret_p95: f32,
}

impl Display for Diagnostics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        const BAR_WIDTH: usize = 50;
        writeln!(f, "== Pool (is the input usable?) ==")?;
        writeln!(f, "male_count: {}", self.male_count)?;
        writeln!(f, "female_count: {}", self.female_count)?;
        writeln!(
            f,
            "pairs_scored: {}\t((male, female) pairs that survived all dealbreakers and got a score)",
            self.pairs_scored
        )?;
        writeln!(
            f,
            "dealbreaker_eliminated: {}\t(pairs rejected before scoring due to dealbreakers)",
            self.dealbreaker_eliminated
        )?;
        writeln!(
            f,
            "  by wants_children: {}",
            self.dealbreaker_by_wants_children
        )?;
        writeln!(f, "  by stay_local: {}", self.dealbreaker_by_stay_local)?;
        writeln!(
            f,
            "  by marriage_timeline: {}",
            self.dealbreaker_by_marriage_timeline
        )?;
        writeln!(f, "  by religion: {}", self.dealbreaker_by_religion)?;

        writeln!(f, "\n== Convergence (did the algorithm finish cleanly?) ==")?;
        writeln!(
            f,
            "cap_relaxed: {}\t(true if the appearance cap had to be raised to make progress; true = pool was tight and quality may have suffered)",
            self.cap_relaxed
        )?;
        writeln!(
            f,
            "shortlist_full: {}\t(people whose shortlist reached the target size)",
            self.shortlist_full
        )?;
        writeln!(
            f,
            "shortlist_acceptable: {}\t(people with shortlists at or above the minimum but not full; some quality loss)",
            self.shortlist_acceptable
        )?;
        writeln!(
            f,
            "shortlist_under_min: {}\t(people whose shortlists are below the minimum acceptable size; the relaxed retry couldn't fill them)",
            self.shortlist_under_min
        )?;
        writeln!(
            f,
            "shortlist_empty: {}\t(people who got no matches as a subject; usually means no opposite-gender candidates passed their dealbreakers)",
            self.shortlist_empty
        )?;
        writeln!(
            f,
            "appearance_max: {}\t(the most times any one person was picked; should sit at or just below the appearance cap when load is balanced)",
            self.appearance_max
        )?;
        writeln!(
            f,
            "appearance_stddev: {:.2}\t(spread of pick counts; low = even distribution, high = a few popular people absorbed many picks while others got none)",
            self.appearance_stddev
        )?;
        writeln!(
            f,
            "zero_appearance_participants: {}\t(people no one's shortlist included (object side); distinct from shortlist_empty (subject side))",
            self.zero_appearance_participants
        )?;

        writeln!(f, "\n== Quality (is the output good?) ==")?;
        writeln!(
            f,
            "shortlisted_score_histogram: (distribution of scores that were actually served; mass in high buckets is healthy, weight in low buckets means someone got a poor match)"
        )?;
        let labels = [
            "<0.5   ", "0.5-0.6", "0.6-0.7", "0.7-0.8", "0.8-0.9", ">=0.9  ",
        ];
        let max_count = *self.histogram.iter().max().unwrap_or(&1);
        for (label, &count) in labels.iter().zip(self.histogram.iter()) {
            let bar_len = if max_count == 0 {
                0
            } else {
                count * BAR_WIDTH / max_count
            };
            let bar: String = "#".repeat(bar_len);
            writeln!(f, "  {label} | {bar:<BAR_WIDTH$}  {count}")?;
        }
        writeln!(
            f,
            "regret_mean: {:.3}\t(average gap between each person's best possible score and the best score they were actually served; 0 = everyone got their algorithmic best, larger = cap or shuffle pushed them away)",
            self.regret_mean
        )?;
        writeln!(
            f,
            "regret_p95: {:.3}\t(95th percentile of that gap; catches the worst cases the mean hides — a large p95 with a small mean means a few people were significantly downgraded)",
            self.regret_p95
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum DealbreakerCause {
    WantsChildren,
    StayLocal,
    MarriageTimeline,
    Religion,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Matches {
    cards: Vec<MatchCard>,
    print_scores: bool,
}

impl Display for Matches {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for card in &self.cards {
            writeln!(f, "{} ({})\n\nMatches:", card.name, card.email)?;
            for (index, m) in card.shortlist.iter().enumerate() {
                if self.print_scores {
                    writeln!(f, "\t{} ({}) ({})", m.name, m.email, m.score)?;
                } else {
                    writeln!(f, "\t{} ({})", m.name, m.email)?;
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

#[derive(Clone, Debug, PartialEq)]
pub struct MatchCard {
    name: String,
    email: String,
    shortlist: Vec<ShortlistMatch>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShortlistMatch {
    name: String,
    email: String,
    freeresponse: FreeResponse,
    score: f32,
}

pub fn create_matches(
    responses: &[QuestionnaireResponse],
    sort_shortlists_by_score: bool,
    print_scores: bool,
    collect_diagnostics: bool,
    target_shortlist: usize,
    min_shortlist: usize,
    max_appearances: usize,
    max_appearances_relaxed: usize,
) -> (Matches, Option<Diagnostics>) {
    let mut rng = rand::rng();

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
        &mut rng,
        collect_diagnostics,
        target_shortlist,
        min_shortlist,
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
                matches.shuffle(&mut rng);
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
                            email: match_response.demographics.email.clone(),
                            freeresponse: match_response.freeresponse.clone(),
                            score: matched_score,
                        }
                    })
                    .collect_vec(),
            }
        })
        .collect_vec();

    matches.sort_unstable_by(|a, b| a.email.cmp(&b.email));

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
        min_shortlist,
    );

    (result, diagnostics)
}

fn build_diagnostics(
    pairs_stats: Option<PairsStats>,
    shortlist_stats: Option<ShortlistStats>,
    ids: &[&str],
    pairs: &FxHashMap<(&str, &str), f32>,
    result: &Matches,
    target_shortlist: usize,
    min_shortlist: usize,
) -> Option<Diagnostics> {
    let (ps, ss) = pairs_stats.zip(shortlist_stats)?;
    let total_participants = ids.len();

    // Convergence: shortlist size buckets
    let mut shortlist_full = 0usize;
    let mut shortlist_acceptable = 0usize;
    let mut shortlist_under_min = 0usize;
    let mut shortlist_empty = 0usize;
    for card in &result.cards {
        match card.shortlist.len() {
            n if n >= target_shortlist => shortlist_full += 1,
            n if n >= min_shortlist => shortlist_acceptable += 1,
            0 => shortlist_empty += 1,
            _ => shortlist_under_min += 1,
        }
    }

    // Appearance stats
    let appearance_max = ss.appearance_count.values().copied().max().unwrap_or(0);
    let zero_appearance_participants = ids
        .iter()
        .filter(|id| ss.appearance_count.get(**id).copied().unwrap_or(0) == 0)
        .count();
    let counts_with_zeros: Vec<f32> = ids
        .iter()
        .map(|id| ss.appearance_count.get(*id).copied().unwrap_or(0) as f32)
        .collect();
    let mean_appearances = counts_with_zeros.iter().sum::<f32>() / total_participants.max(1) as f32;
    let variance = counts_with_zeros
        .iter()
        .map(|&c| (c - mean_appearances).powi(2))
        .sum::<f32>()
        / total_participants.max(1) as f32;
    let appearance_stddev = variance.sqrt();

    // Quality: histogram of shortlisted scores
    let mut histogram = [0usize; 6];
    for card in &result.cards {
        for m in &card.shortlist {
            let idx = match m.score {
                s if s < 0.5 => 0,
                s if s < 0.6 => 1,
                s if s < 0.7 => 2,
                s if s < 0.8 => 3,
                s if s < 0.9 => 4,
                _ => 5,
            };
            histogram[idx] += 1;
        }
    }

    // Regret: gap between each person's best possible pool score and their best served score.
    // Includes people with empty shortlists who had pool candidates (regret = top_pool - 0).
    // Only excludes people with no pool candidates at all (not in top_pool_scores).
    let mut regrets: Vec<f32> = result
        .cards
        .iter()
        .filter_map(|card| {
            let top_pool = *ps.top_pool_scores.get(&card.email)?;
            let top_served = card
                .shortlist
                .iter()
                .map(|m| m.score)
                .fold(0.0_f32, f32::max);
            Some((top_pool - top_served).max(0.0))
        })
        .collect();
    let regret_mean = if regrets.is_empty() {
        0.0
    } else {
        regrets.iter().sum::<f32>() / regrets.len() as f32
    };
    let regret_p95 = if regrets.is_empty() {
        0.0
    } else {
        let p95_idx = (regrets.len() * 95)
            .div_ceil(100)
            .saturating_sub(1)
            .min(regrets.len() - 1);
        regrets.select_nth_unstable_by(p95_idx, |a, b| a.partial_cmp(b).expect("comparable"));
        regrets[p95_idx]
    };

    Some(Diagnostics {
        male_count: ps.male_count,
        female_count: ps.female_count,
        pairs_scored: pairs.len(),
        dealbreaker_eliminated: ps.dealbreaker_eliminated,
        dealbreaker_by_wants_children: ps.dealbreaker_by_wants_children,
        dealbreaker_by_stay_local: ps.dealbreaker_by_stay_local,
        dealbreaker_by_marriage_timeline: ps.dealbreaker_by_marriage_timeline,
        dealbreaker_by_religion: ps.dealbreaker_by_religion,
        cap_relaxed: ss.cap_relaxed,
        shortlist_full,
        shortlist_acceptable,
        shortlist_under_min,
        shortlist_empty,
        appearance_max,
        appearance_stddev,
        zero_appearance_participants,
        histogram,
        regret_mean,
        regret_p95,
    })
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
    // Reduced, scored twice via Partner Preferences cross-match below (TODO: Is this true?)
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

struct PairsStats {
    male_count: usize,
    female_count: usize,
    dealbreaker_eliminated: usize,
    dealbreaker_by_wants_children: usize,
    dealbreaker_by_stay_local: usize,
    dealbreaker_by_marriage_timeline: usize,
    dealbreaker_by_religion: usize,
    top_pool_scores: FxHashMap<String, f32>,
}

/// Builds a hashmap of (male.id, female.id) -> score. No pairing will be in the map if
/// there are dealbreakers on either side. Score takes into account how compatible each
/// side is with the other, so no need for a reverse map (female.id, male.id) -> score.
fn build_scored_pairs(
    responses: &[QuestionnaireResponse],
    collect_stats: bool,
) -> (FxHashMap<(&str, &str), f32>, Option<PairsStats>) {
    // Empircally good enough for 1000 people. Technically the capacity should be (responses / 2) ^ 2 assuming
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
            top_pool_scores: FxHashMap::default(),
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
                            s.dealbreaker_by_marriage_timeline += 1
                        }
                        DealbreakerCause::Religion => s.dealbreaker_by_religion += 1,
                    }
                }
                continue;
            }

            let score = mutual_score(male, female);
            pairs.insert((male.id(), female.id()), score);

            if let Some(ref mut s) = stats {
                let e = s
                    .top_pool_scores
                    .entry(male.id().to_string())
                    .or_insert(0.0_f32);
                if score > *e {
                    *e = score;
                }
                let e = s
                    .top_pool_scores
                    .entry(female.id().to_string())
                    .or_insert(0.0_f32);
                if score > *e {
                    *e = score;
                }
            }
        }
    }

    (pairs, stats)
}

struct ShortlistStats {
    cap_relaxed: bool,
    appearance_count: FxHashMap<String, usize>,
}

type Shortlists = FxHashMap<String, Vec<(String, f32)>>;

fn assign_shortlists(
    ids: &[&str],
    pairs: &FxHashMap<(&str, &str), f32>,
    rng: &mut ThreadRng,
    collect_stats: bool,
    target_shortlist: usize,
    min_shortlist: usize,
    max_appearances: usize,
    max_appearances_relaxed: usize,
) -> (Shortlists, Option<ShortlistStats>) {
    let mut cap = max_appearances;
    let mut cap_relaxed = false;
    let mut appearance_count: FxHashMap<&str, usize> = FxHashMap::default();
    let mut shortlists: FxHashMap<String, Vec<(String, f32)>> = FxHashMap::default();

    // # Precompute each person's ranked candidate list (descending score)
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

    let mut incomplete: FxHashSet<_> = ids.iter().collect();

    while !incomplete.is_empty() {
        let mut order: Vec<_> = incomplete.iter().copied().collect();
        order.shuffle(rng);

        let mut made_progress = false;
        for pid in order {
            if shortlists
                .get(*pid)
                .is_some_and(|a| a.len() >= target_shortlist)
            {
                incomplete.remove(&pid);
                continue;
            }

            if let Some((other_id, score)) =
                next_available(pid, cap, &ranked_candidates, &shortlists, &appearance_count)
            {
                shortlists
                    .entry(pid.to_string())
                    .or_insert(Vec::with_capacity(target_shortlist))
                    .push((other_id.to_string(), score));
                *appearance_count.entry(other_id).or_default() += 1;
                made_progress = true;
            }
        }

        // If no progress was made this round, relax cap and retry until exhausted.
        if !made_progress {
            cap = max_appearances_relaxed;
            loop {
                let mut made_progress_relaxed = false;
                let mut order: Vec<_> = incomplete.iter().copied().collect();
                order.shuffle(rng);
                for pid in order {
                    if shortlists
                        .get(*pid)
                        .is_some_and(|a| a.len() >= min_shortlist)
                    {
                        incomplete.remove(&pid);
                        continue;
                    }
                    if let Some((other_id, score)) =
                        next_available(pid, cap, &ranked_candidates, &shortlists, &appearance_count)
                    {
                        shortlists
                            .entry(pid.to_string())
                            .or_insert(Vec::with_capacity(min_shortlist))
                            .push((other_id.to_string(), score));
                        *appearance_count.entry(other_id).or_default() += 1;
                        made_progress_relaxed = true;
                        cap_relaxed = true;
                    }
                }
                if !made_progress_relaxed {
                    break;
                }
            }
            break;
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
) -> Option<(&'a str, f32)> {
    for (other_id, score) in &ranked_candidates[pid] {
        if shortlists
            .get(pid)
            .is_some_and(|sl| sl.iter().any(|i| &i.0 == other_id))
        {
            continue;
        }
        if appearance_count.get(other_id).copied().unwrap_or(0) >= current_cap {
            continue;
        }
        return Some((other_id, *score));
    }

    None
}

#[cfg(test)]
mod tests {
    use crate::parsing::{Dealbreakers, Demographics, MyReligiousCommitment};

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
        let (matches, _) = create_matches(&[], true, true, false, 5, 3, 12, 14);
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
        let (matches, _) = create_matches(
            &[QuestionnaireResponse::default()],
            true,
            true,
            false,
            5,
            3,
            12,
            14,
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
                email: "first".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let second = QuestionnaireResponse {
            demographics: Demographics {
                email: "second".to_string(),
                gender: Gender::Female,
                ..Default::default()
            },
            ..Default::default()
        };
        let (matches, _) = create_matches(&[first, second], true, true, false, 5, 3, 12, 14);
        assert_eq!(
            matches,
            Matches {
                cards: vec![
                    MatchCard {
                        name: "".to_string(),
                        email: "first".to_string(),
                        shortlist: vec![ShortlistMatch {
                            name: "".to_string(),
                            email: "second".to_string(),
                            freeresponse: FreeResponse { responses: vec![] },
                            score: 1.0
                        }]
                    },
                    MatchCard {
                        name: "".to_string(),
                        email: "second".to_string(),
                        shortlist: vec![ShortlistMatch {
                            name: "".to_string(),
                            email: "first".to_string(),
                            freeresponse: FreeResponse { responses: vec![] },
                            score: 1.0
                        }]
                    }
                ],
                print_scores: true
            }
        )
    }
}

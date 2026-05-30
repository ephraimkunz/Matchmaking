use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
};

use itertools::Itertools;
use rand::prelude::*;

use crate::parsing::{
    Age, FreeResponse, Gender, MarriageTimelineResponse, PartnersReligionResponse,
    QuestionnaireResponse, YesNoMaybeResponse,
};

use anyhow::Result;

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
) -> Matches {
    let mut rng = rand::rng();

    // Score all pairs
    let pairs = build_scored_pairs(responses);

    // Assign shortlists via round-robin
    let ids = responses
        .iter()
        .map(QuestionnaireResponse::id)
        .collect_vec();
    let shortlists = assign_shortlists(&ids, &pairs, &mut rng);

    let mut matches = shortlists
        .into_iter()
        .map(|(id, matches)| {
            let response = responses
                .iter()
                .find(|i| i.id() == id)
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
                        let match_response = responses
                            .iter()
                            .find(|i| i.id() == matched_id)
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

    matches.sort_unstable_by_key(|m| m.email.clone());

    Matches {
        cards: matches,
        print_scores,
    }
}

/// Returns true of there are no dealbreakers a -> b, or b -> a, otherwise returns false.
fn passes_dealbreakers(a: &QuestionnaireResponse, b: &QuestionnaireResponse) -> bool {
    match (
        &a.dealbreakers.wants_children,
        &b.dealbreakers.wants_children,
    ) {
        (YesNoMaybeResponse::No, YesNoMaybeResponse::Yes)
        | (YesNoMaybeResponse::Yes, YesNoMaybeResponse::No) => return false,
        _ => (),
    }

    match (&a.dealbreakers.stay_local, &b.dealbreakers.stay_local) {
        (YesNoMaybeResponse::No, YesNoMaybeResponse::Yes)
        | (YesNoMaybeResponse::Yes, YesNoMaybeResponse::No) => return false,
        _ => (),
    }

    match (
        &a.dealbreakers.marriage_timeline,
        &b.dealbreakers.marriage_timeline,
    ) {
        (MarriageTimelineResponse::ZeroToTwo, MarriageTimelineResponse::FivePlus)
        | (MarriageTimelineResponse::FivePlus, MarriageTimelineResponse::ZeroToTwo) => {
            return false;
        }
        _ => (),
    }

    match &a.dealbreakers.partners_religious_commitment {
        PartnersReligionResponse::Same
            if a.dealbreakers.my_religious_commitment.0
                != b.dealbreakers.my_religious_commitment.0 =>
        {
            return false;
        }
        PartnersReligionResponse::Within1Level
            if a.dealbreakers
                .my_religious_commitment
                .0
                .abs_diff(b.dealbreakers.my_religious_commitment.0)
                > 1 =>
        {
            return false;
        }
        _ => (),
    }

    match &b.dealbreakers.partners_religious_commitment {
        PartnersReligionResponse::Same
            if b.dealbreakers.my_religious_commitment.0
                != a.dealbreakers.my_religious_commitment.0 =>
        {
            return false;
        }
        PartnersReligionResponse::Within1Level
            if a.dealbreakers
                .my_religious_commitment
                .0
                .abs_diff(b.dealbreakers.my_religious_commitment.0)
                > 1 =>
        {
            return false;
        }
        _ => (),
    }

    true
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

    let relationsip_dynamics_results =
        process_relationship_dynamics(a, b, subject_chosen_weight_scale_factor);
    total += relationsip_dynamics_results.0;
    weight_sum += relationsip_dynamics_results.1;

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
fn build_scored_pairs(responses: &[QuestionnaireResponse]) -> HashMap<(&str, &str), f32> {
    // Empircally good enough for 1000 people. Technically the capacity should be (responses / 2) ^ 2 assuming
    // equal numbers of men and women. But dealbreakers shrink that down.
    let mut pairs = HashMap::with_capacity(50000);

    for male in responses
        .iter()
        .filter(|r| r.demographics.gender == Gender::Male)
    {
        for female in responses
            .iter()
            .filter(|r| r.demographics.gender == Gender::Female)
        {
            if !passes_dealbreakers(male, female) {
                continue;
            }

            let score = mutual_score(male, female);

            pairs.insert((male.id(), female.id()), score);
        }
    }

    pairs
}

fn assign_shortlists(
    ids: &[&str],
    pairs: &HashMap<(&str, &str), f32>,
    rng: &mut ThreadRng,
) -> HashMap<String, Vec<(String, f32)>> {
    const MAX_APPEARANCES: u8 = 12;
    const TARGET_SHORTLIST: u8 = 5;

    let mut cap = MAX_APPEARANCES;
    let mut appearance_count: HashMap<&str, u8> = HashMap::new();
    let mut shortlists: HashMap<String, Vec<(String, f32)>> = HashMap::new();

    // # Precompute each person's ranked candidate list (descending score)
    let mut ranked_candidates: HashMap<&str, Vec<(&str, f32)>> = HashMap::new();

    for ((a, b), &s) in pairs {
        ranked_candidates.entry(a).or_default().push((b, s));
        ranked_candidates.entry(b).or_default().push((a, s));
    }

    // Ensure every id has an entry, even people who are not in pairs (only one of their gender, or dealbreakers that exclude everyone else)
    for pid in ids {
        ranked_candidates.entry(pid).or_default();
    }

    for value in ranked_candidates.values_mut() {
        // Reverse sort, so largest scores come first.
        value.sort_unstable_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .expect("Should be able to compare floats")
        });
    }

    let mut incomplete: HashSet<_> = ids.iter().collect();

    while !incomplete.is_empty() {
        let mut order: Vec<_> = incomplete.iter().copied().collect();
        order.shuffle(rng);

        let mut made_progress = false;
        for pid in order {
            if shortlists
                .get(*pid)
                .is_some_and(|a| a.len() >= TARGET_SHORTLIST as usize)
            {
                incomplete.remove(&pid);
                continue;
            }

            if let Some((other_id, score)) =
                next_available(pid, cap, &ranked_candidates, &shortlists, &appearance_count)
            {
                shortlists
                    .entry(pid.to_string())
                    .or_insert(Vec::with_capacity(TARGET_SHORTLIST.into()))
                    .push((other_id.to_string(), score));
                *appearance_count.entry(other_id).or_default() += 1;
                made_progress = true;
            }
        }

        // If no progress was made this round, relax cap and retry.
        if !made_progress {
            const MAX_APPEARANCES_RELAXED: u8 = 14;
            cap = MAX_APPEARANCES_RELAXED;
            let mut order: Vec<_> = incomplete.iter().copied().collect();
            order.shuffle(rng);
            for pid in order {
                const MIN_SHORTLIST: u8 = 3;
                if shortlists
                    .get(*pid)
                    .is_some_and(|a| a.len() >= MIN_SHORTLIST as usize)
                {
                    incomplete.remove(&pid);
                    continue;
                }
                if let Some((other_id, score)) =
                    next_available(pid, cap, &ranked_candidates, &shortlists, &appearance_count)
                {
                    shortlists
                        .entry(pid.to_string())
                        .or_insert(Vec::with_capacity(TARGET_SHORTLIST.into()))
                        .push((other_id.to_string(), score));
                    *appearance_count.entry(other_id).or_default() += 1;
                }
            }
            // Break after relaxed retry regardless, avoid infinite loop
            break;
        }
    }

    shortlists
}

fn next_available<'a>(
    pid: &str,
    current_cap: u8,
    ranked_candidates: &'a HashMap<&str, Vec<(&str, f32)>>,
    shortlists: &HashMap<String, Vec<(String, f32)>>,
    appearance_count: &HashMap<&str, u8>,
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
        assert!(!passes_dealbreakers(&a, &b));
        assert!(!passes_dealbreakers(&b, &a));
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
        assert!(passes_dealbreakers(&a, &a));
        assert!(passes_dealbreakers(&b, &b));
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
        assert!(passes_dealbreakers(&a, &c));
        assert!(passes_dealbreakers(&c, &a));
        assert!(passes_dealbreakers(&b, &c));
        assert!(passes_dealbreakers(&c, &b));
        assert!(passes_dealbreakers(&c, &c));
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
        assert!(!passes_dealbreakers(&a, &b));
        assert!(!passes_dealbreakers(&b, &a));
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
        assert!(passes_dealbreakers(&a, &a));
        assert!(passes_dealbreakers(&b, &b));
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
        assert!(passes_dealbreakers(&a, &c));
        assert!(passes_dealbreakers(&c, &a));
        assert!(passes_dealbreakers(&b, &c));
        assert!(passes_dealbreakers(&c, &b));
        assert!(passes_dealbreakers(&c, &c));
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
        assert!(!passes_dealbreakers(&a, &b));
        assert!(!passes_dealbreakers(&b, &a));
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
        assert!(passes_dealbreakers(&a, &a));
        assert!(passes_dealbreakers(&b, &b));
        assert!(passes_dealbreakers(&c, &c));
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
        assert!(passes_dealbreakers(&a, &b));
        assert!(passes_dealbreakers(&b, &c));
        assert!(passes_dealbreakers(&c, &b));
        assert!(passes_dealbreakers(&b, &a));
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

        assert!(passes_dealbreakers(&a, &b));
        assert!(passes_dealbreakers(&b, &a));
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

        assert!(passes_dealbreakers(&a, &b));
        assert!(passes_dealbreakers(&b, &a));

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

        assert!(passes_dealbreakers(&a, &b));
        assert!(passes_dealbreakers(&b, &a));

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

        assert!(passes_dealbreakers(&a, &b));
        assert!(passes_dealbreakers(&b, &a));
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

        assert!(!passes_dealbreakers(&a, &b));
        assert!(!passes_dealbreakers(&b, &a));

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

        assert!(!passes_dealbreakers(&a, &b));
        assert!(!passes_dealbreakers(&b, &a));
    }

    #[test]
    fn test_default_mutual_score() {
        let default = QuestionnaireResponse::default();
        let mutual_score = mutual_score(&default, &default);
        assert_eq!(mutual_score, 1.0)
    }

    #[test]
    fn test_empty_create_matches() {
        let matches = create_matches(&[], true, true);
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
        let matches = create_matches(&[QuestionnaireResponse::default()], true, true);
        assert_eq!(
            matches,
            Matches {
                cards: vec![],
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
        let matches = create_matches(&[first, second], true, true);
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

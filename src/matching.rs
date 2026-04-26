use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use crate::parsing::{
    FreeResponse, Gender, MarriageTimelineResponse, PartnersReligionResponse,
    QuestionnaireResponse, YesNoMaybeResponse,
};
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct Matches(Vec<MatchCard>);

impl Display for Matches {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for card in &self.0 {
            writeln!(
                f,
                "======================\n{} ({})\n\nMatches:",
                card.name, card.email
            )?;
            for (index, m) in card.shortlist.iter().enumerate() {
                writeln!(f, "\t{} ({}) ({})", m.name, m.email, m.score)?;

                for (k, v) in &m.freeresponse.responses {
                    writeln!(f, "\t{} {}", k, v)?;
                }

                if index < (card.shortlist.len() - 1) {
                    writeln!(f)?;
                }
            }

            writeln!(f, "======================\n")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct MatchCard {
    name: String,
    email: String,
    shortlist: Vec<ShortlistMatch>,
}

#[derive(Clone, Debug)]
pub struct ShortlistMatch {
    name: String,
    email: String,
    freeresponse: FreeResponse,
    score: f32,
}

pub fn create_matches(responses: &[QuestionnaireResponse]) -> Result<Matches> {
    // Score all pairs
    let pairs = build_scored_pairs(responses.to_vec());

    // Assign shortlists via round-robin
    // let shortlists = assign_shortlists(responses, pairs);

    Ok(Matches(vec![
        MatchCard {
            name: "Ephraim Kunz".to_string(),
            email: "ephraimkunz@me.com".to_string(),
            shortlist: vec![
                ShortlistMatch {
                    name: "Ashlee Hendricks".to_string(),
                    email: "ashbegash@gmail.com".to_string(),
                    freeresponse: FreeResponse {
                        responses: [
                            ("My favorite cat is".to_string(), "meow".to_string()),
                            ("I like to: ".to_string(), "eat".to_string()),
                        ]
                        .into_iter()
                        .collect(),
                    },
                    score: 0.65,
                },
                ShortlistMatch {
                    name: "Ashlee Hendricks".to_string(),
                    email: "ashbegash@gmail.com".to_string(),
                    freeresponse: FreeResponse {
                        responses: [("My favorite cat is".to_string(), "meow".to_string())]
                            .into_iter()
                            .collect(),
                    },
                    score: 0.65,
                },
            ],
        },
        MatchCard {
            name: "Ephraim Kunz".to_string(),
            email: "ephraimkunz@me.com".to_string(),
            shortlist: vec![
                ShortlistMatch {
                    name: "Ashlee Hendricks".to_string(),
                    email: "ashbegash@gmail.com".to_string(),
                    freeresponse: FreeResponse {
                        responses: [
                            ("My favorite cat is".to_string(), "meow".to_string()),
                            ("I like to: ".to_string(), "eat".to_string()),
                        ]
                        .into_iter()
                        .collect(),
                    },
                    score: 0.65,
                },
                ShortlistMatch {
                    name: "Ashlee Hendricks".to_string(),
                    email: "ashbegash@gmail.com".to_string(),
                    freeresponse: FreeResponse {
                        responses: [("My favorite cat is".to_string(), "meow".to_string())]
                            .into_iter()
                            .collect(),
                    },
                    score: 0.65,
                },
            ],
        },
    ]))
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
            if !(a.dealbreakers.my_religious_commitment.0 - 1
                ..=a.dealbreakers.my_religious_commitment.0 + 1)
                .contains(&b.dealbreakers.my_religious_commitment.0) =>
        {
            return false;
        }
        _ => (),
    };

    match &b.dealbreakers.partners_religious_commitment {
        PartnersReligionResponse::Same
            if b.dealbreakers.my_religious_commitment.0
                != a.dealbreakers.my_religious_commitment.0 =>
        {
            return false;
        }
        PartnersReligionResponse::Within1Level
            if !(b.dealbreakers.my_religious_commitment.0 - 1
                ..=b.dealbreakers.my_religious_commitment.0 + 1)
                .contains(&a.dealbreakers.my_religious_commitment.0) =>
        {
            return false;
        }
        _ => (),
    };

    true
}

/// Calculated the mutual score of a and b in a non-directional way, so that a's compatibility
/// with b and b's compatibility with a are both contained in a single score.
fn mutual_score(a: &QuestionnaireResponse, b: &QuestionnaireResponse) -> f32 {
    todo!()
}

/// Builds a hashmap of (male.id, female.id) -> score. No pairing will be in the map if
/// there are dealbreakers on either side. Score takes into account how compatible each
/// side is with the other, so no need for a reverse map (female.id, male.id) -> score.
fn build_scored_pairs(responses: Vec<QuestionnaireResponse>) -> HashMap<(String, String), f32> {
    let mut pairs = HashMap::new();

    let (males, females): (Vec<QuestionnaireResponse>, Vec<QuestionnaireResponse>) = responses
        .into_iter()
        .partition(|r| match r.demographics.gender {
            Gender::Male => true,
            Gender::Female => false,
        });

    for male in &males {
        for female in &females {
            if !passes_dealbreakers(male, female) {
                continue;
            }

            let score = mutual_score(male, female);

            pairs.insert((male.id(), female.id()), score);
        }
    }

    pairs
}

#[cfg(test)]
mod tests {
    use crate::parsing::{Dealbreakers, MyReligiousCommitment};

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
}

use std::fmt::{Display, Formatter};

use crate::parsing::{FreeResponse, QuestionnaireResponse};
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
    Ok(Matches(vec![MatchCard {
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
    }, MatchCard {
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
    }]))
}

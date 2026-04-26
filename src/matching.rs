use std::fmt::{Display, Formatter};

use crate::parsing::QuestionnaireResponse;
use anyhow::Result;

pub struct Matches {}

impl Display for Matches {
    fn fmt(&self, _: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        Ok(())
    }
}

pub fn create_matches(responses: &[QuestionnaireResponse]) -> Result<Matches> {
    Ok(Matches {})
}

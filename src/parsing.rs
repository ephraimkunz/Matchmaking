use anyhow::{Context, Result, anyhow, bail, ensure};
use itertools::Itertools;
use std::{collections::HashMap, vec};

#[derive(Clone, Debug, Default)]
pub struct QuestionnaireResponse {
    pub demographics: Demographics,
    pub dealbreakers: Dealbreakers,
    pub corevalues: CoreValues,
    pub relationshipdynamics: RelationshipDynamics,
    pub lifestylemoney: LifestyleMoney,
    pub selfdescription: SelfDescription,
    pub partnerpreferences: PartnerPreferences,
    pub socialstyle: SocialStyle,
    pub interests: Interests,
    pub freeresponse: FreeResponse,
}

impl QuestionnaireResponse {
    pub fn id(&self) -> String {
        self.demographics.email.clone()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Demographics {
    pub name: String,
    pub email: String,
    pub gender: Gender,
    pub age: Age,
}

#[derive(Clone, Debug, Default)]
pub struct Dealbreakers {
    pub wants_children: YesNoMaybeResponse,
    pub marriage_timeline: MarriageTimelineResponse,
    pub stay_local: YesNoMaybeResponse,
    pub my_religious_commitment: MyReligiousCommitment,
    pub partners_religious_commitment: PartnersReligionResponse,
}

#[derive(Clone, Debug, Default)]
pub struct CoreValues {
    pub response_and_weights: [ResponseAndWeight; 14],
}

#[derive(Clone, Debug, Default)]
pub struct RelationshipDynamics {
    pub response_and_weights: [ResponseAndWeight; 8],
    pub responses: [FourChoiceResponse; 3],
}

#[derive(Clone, Debug, Default)]
pub struct LifestyleMoney {
    pub responses: [FourChoiceResponse; 8],
    pub num_children: NumChildren,
}

#[derive(Clone, Debug, Default)]
pub struct SelfDescription {
    /// Crossmatched against corresponding item in PartnerPreferences.crossmatched
    pub crossmatched: [FourChoiceResponse; 8],

    /// Direct comparison with corresponding SelfDescription.direct for partner candidate.
    pub direct: [FourChoiceResponse; 7],
}

#[derive(Clone, Debug, Default)]
pub struct PartnerPreferences {
    /// Crossmatched against corresponding item in SelfDescription.crossmatched
    pub crossmatched: [FourChoiceResponse; 8],

    /// Direct comparison with corresponding PartnerPreferences.direct for partner candidate.
    pub direct: [FourChoiceResponse; 2],
}

#[derive(Clone, Debug, Default)]
pub struct SocialStyle {
    pub responses: [FourChoiceResponse; 8],
}

#[derive(Clone, Debug, Default)]
pub struct Interests {
    pub responses: [FourChoiceResponse; 8],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FreeResponse {
    pub responses: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct Age(pub u8);

impl Age {
    pub fn new(string: &str) -> Result<Self> {
        let age: u8 = string.parse().context("Unable to parse age to number")?;
        ensure!(
            (Self::MIN_AGE..=Self::MAX_AGE).contains(&age),
            "Age {age} is not in the correct range"
        );
        Ok(Self(age))
    }

    pub const MIN_AGE: u8 = 26;
    pub const MAX_AGE: u8 = 37;
}

impl Default for Age {
    fn default() -> Self {
        Self(26)
    }
}

#[derive(Clone, Debug)]
pub struct MyReligiousCommitment(pub u8);

impl MyReligiousCommitment {
    pub fn new(string: &str) -> Result<Self> {
        let my_religious_commitment: u8 = string
            .parse()
            .context("Unable to parse my religious commitment to number")?;
        ensure!(
            (1..=5).contains(&my_religious_commitment),
            "My religious commitment {my_religious_commitment} is not in the correct range"
        );

        Ok(Self(my_religious_commitment))
    }
}

impl Default for MyReligiousCommitment {
    fn default() -> Self {
        Self(1)
    }
}

#[derive(Clone, Debug)]
pub struct NumChildren(pub u8);

impl NumChildren {
    fn new(string: &str) -> Result<Self> {
        let num_children: u8 = string
            .parse()
            .context("Unable to parse num children to number")?;
        ensure!(
            (0..=9).contains(&num_children),
            "Num children {num_children} is not in the correct range"
        );

        Ok(Self(num_children))
    }

    /// Score from 0 to 1.
    pub fn normalized(&self) -> f32 {
        (self.0) as f32 / 9.0
    }
}

impl Default for NumChildren {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Clone, Debug)]
pub struct FourChoiceResponse(pub u8);

impl FourChoiceResponse {
    fn new(string: &str) -> Result<Self> {
        let response = string
            .parse()
            .context("Unable to parse question response to number")?;
        ensure!(
            (1..=4).contains(&response),
            "Response {response} is not in the correct range"
        );

        Ok(Self(response))
    }

    /// Score from 0 to 1.
    pub fn normalized(&self) -> f32 {
        (self.0 - 1) as f32 / 3.0
    }
}

impl Default for FourChoiceResponse {
    fn default() -> Self {
        Self(1)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ResponseAndWeight {
    pub response: FourChoiceResponse,
    pub weight: FiveChoiceWeight,
}

#[derive(Clone, Debug)]
pub struct FiveChoiceWeight(pub u8);

impl FiveChoiceWeight {
    fn new(string: &str) -> Result<Self> {
        let weight: u8 = match string {
            "I don't care if we agree" => 0,
            "A little" => 1,
            "Somewhat" => 2,
            "Very" => 3,
            "We MUST agree on this" => 4,
            other => bail!(
                "Unexpected imporance weight \"{}\", expected \"I don't care if we agree\" or \"A little\" or \"Somewhat\" or \"Very\" or \"We MUST agree on this\"",
                other
            ),
        };
        ensure!(
            (0..=4).contains(&weight),
            "Weight {weight} is not in the correct range"
        );

        Ok(Self(weight))
    }

    const MAX_WEIGHT: u8 = 4;
    const MIN_NORMALIZED: f32 = 0.25;
    const MAX_NORMALIZED: f32 = 2.0;

    /// Take the 0, 1, 2, 3, or 4 discrete response and map it to MIN_NORMALIZED - MAX_NORMALIZED
    pub fn normalized(&self) -> f32 {
        Self::MIN_NORMALIZED
            + self.0 as f32 / Self::MAX_WEIGHT as f32
                * (Self::MAX_NORMALIZED - Self::MIN_NORMALIZED)
    }
}

impl Default for FiveChoiceWeight {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Clone, Debug, Default)]
pub enum Gender {
    #[default]
    Male,
    Female,
}

#[derive(Clone, Debug, Default)]
pub enum YesNoMaybeResponse {
    #[default]
    Yes,
    No,
    Maybe,
}

#[derive(Clone, Debug, Default)]
pub enum MarriageTimelineResponse {
    #[default]
    ZeroToTwo,
    TwoToFive,
    FivePlus,
}

#[derive(Clone, Debug, Default)]
pub enum PartnersReligionResponse {
    #[default]
    Same,
    Within1Level,
    DoesNotMatter,
}

pub fn parse_responses<R: std::io::Read>(
    reader: &mut csv::Reader<R>,
) -> Result<Vec<QuestionnaireResponse>> {
    let headers = reader.headers().context("Invalid csv header")?.clone();
    let mut responses = vec![];

    for (row_num, result) in reader.records().enumerate() {
        let record = result.with_context(|| format!("Invalid record at row {}", row_num + 2))?; // +2 for 1-index + header row
        let mut header_and_field = headers.into_iter().zip(&record);

        let demographics = parse_demographics(&mut header_and_field)
            .with_context(|| format!("Row {}: failed parsing demographics", row_num + 2))?;

        let dealbreakers = parse_dealbreakers(&mut header_and_field)
            .with_context(|| format!("Row {}: failed parsing dealbreakers", row_num + 2))?;

        let corevalues = parse_corevalues(&mut header_and_field)
            .with_context(|| format!("Row {}: failed parsing corevalues", row_num + 2))?;

        let relationshipdynamics = parse_relationshipdynamics(&mut header_and_field)
            .with_context(|| format!("Row {}: failed parsing relationshipdynamics", row_num + 2))?;

        let lifestylemoney = parse_lifestylemoney(&mut header_and_field)
            .with_context(|| format!("Row {}: failed parsing lifestylemoney", row_num + 2))?;

        let selfdescription = parse_selfdescription(&mut header_and_field)
            .with_context(|| format!("Row {}: failed parsing selfdescription", row_num + 2))?;

        let partnerpreferences = parse_partnerpreferences(&mut header_and_field)
            .with_context(|| format!("Row {}: failed parsing partnerpreferences", row_num + 2))?;

        let socialstyle = parse_socialstyle(&mut header_and_field)
            .with_context(|| format!("Row {}: failed parsing socialstyle", row_num + 2))?;

        let interests = parse_interests(&mut header_and_field)
            .with_context(|| format!("Row {}: failed parsing interests", row_num + 2))?;

        let freeresponse = parse_freeresponse(&mut header_and_field)
            .with_context(|| format!("Row {}: failed parsing freeresponse", row_num + 2))?;

        let last = header_and_field.next();
        ensure!(
            last.is_none(),
            "Unknown extra columns at end of questionnaire: {:?}",
            last
        );

        responses.push(QuestionnaireResponse {
            demographics,
            dealbreakers,
            corevalues,
            relationshipdynamics,
            lifestylemoney,
            selfdescription,
            partnerpreferences,
            socialstyle,
            interests,
            freeresponse,
        })
    }

    Ok(responses)
}

fn parse_demographics<'i>(
    header_and_field: &mut impl Iterator<Item = (&'i str, &'i str)>,
) -> Result<Demographics> {
    let (header, _) = header_and_field
        .next()
        .ok_or_else(|| anyhow!("Unexpected reached the end of csv record"))?;
    ensure!(
        header == "Timestamp",
        "Unexpected header \"{}\", expected \"Timestamp\"",
        header
    );

    let (header, field) = header_and_field
        .next()
        .ok_or_else(|| anyhow!("Unexpected reached the end of csv record"))?;
    ensure!(
        header == "Email Address",
        "Unexpected header \"{}\", expected \"Email Address\"",
        header
    );
    let email = field.to_string();

    let (header, field) = header_and_field
        .next()
        .ok_or_else(|| anyhow!("Unexpected reached the end of csv record"))?;
    ensure!(
        header == "First and last name",
        "Unexpected header \"{}\", expected \"First and last name\"",
        header
    );
    let name = field.to_string();

    let (header, field) = header_and_field
        .next()
        .ok_or_else(|| anyhow!("Unexpected reached the end of csv record"))?;
    ensure!(
        header == "Gender",
        "Unexpected header \"{}\", expected \"Gender\"",
        header
    );
    let gender = match field {
        "Male" => Gender::Male,
        "Female" => Gender::Female,
        other => bail!(
            "Unexpected gender \"{}\", expected \"Male\" or \"Female\"",
            other
        ),
    };

    let (header, field) = header_and_field
        .next()
        .ok_or_else(|| anyhow!("Unexpected reached the end of csv record"))?;
    ensure!(
        header == "Age",
        "Unexpected header \"{}\", expected \"Age\"",
        header
    );
    let age = Age::new(field)?;

    Ok(Demographics {
        name,
        email,
        gender,
        age,
    })
}

fn parse_dealbreakers<'i>(
    header_and_field: &mut impl Iterator<Item = (&'i str, &'i str)>,
) -> Result<Dealbreakers> {
    let (header, field) = header_and_field
        .next()
        .ok_or_else(|| anyhow!("Unexpected reached the end of csv record"))?;
    ensure!(
        header == "I want to have children",
        "Unexpected header \"{}\", expected \"I want to have children\"",
        header
    );
    let wants_children = match field {
        "No" => YesNoMaybeResponse::No,
        "Yes" => YesNoMaybeResponse::Yes,
        "Open to it" => YesNoMaybeResponse::Maybe,
        other => bail!(
            "Unexpected response for marriage timeline \"{}\", expected \"Yes\" or \"No\" or \"Open to it\"",
            other
        ),
    };

    let (header, field) = header_and_field
        .next()
        .ok_or_else(|| anyhow!("Unexpected reached the end of csv record"))?;
    ensure!(
        header == "I'd like to be married within",
        "Unexpected header \"{}\", expected \"I'd like to be married within\"",
        header
    );
    let marriage_timeline = match field {
        "0 - 2 years" => MarriageTimelineResponse::ZeroToTwo,
        "2 - 5 years" => MarriageTimelineResponse::TwoToFive,
        "5+ years" => MarriageTimelineResponse::FivePlus,
        other => bail!(
            "Unexpected response for marriage timeline \"{}\", expected \"0 - 2 years\" or \"2 - 5 years\" or \"5+ years\"",
            other
        ),
    };

    let (header, field) = header_and_field
        .next()
        .ok_or_else(|| anyhow!("Unexpected reached the end of csv record"))?;
    ensure!(
        header == "I intend to stay in Cache Valley long term",
        "Unexpected header \"{}\", expected \"I intend to stay in Cache Valley long term\"",
        header
    );
    let stay_local = match field {
        "Yes" => YesNoMaybeResponse::Yes,
        "No" => YesNoMaybeResponse::No,
        "It depends" => YesNoMaybeResponse::Maybe,
        other => bail!(
            "Unexpected response for stay local \"{}\", expected \"Yes\" or \"No\" or \"It depends\"",
            other
        ),
    };

    let (header, field) = header_and_field
        .next()
        .ok_or_else(|| anyhow!("Unexpected reached the end of csv record"))?;
    ensure!(
        header == "My religious commitment level",
        "Unexpected header \"{}\", expected \"My religious commitment level\"",
        header
    );
    let my_religious_commitment = MyReligiousCommitment::new(field)?;

    let (header, field) = header_and_field
        .next()
        .ok_or_else(|| anyhow!("Unexpected reached the end of csv record"))?;
    ensure!(
        header == "My partner's religious commitment level should be:",
        "Unexpected header \"{}\", expected \"My partner's religious commitment level should be:\"",
        header
    );
    let partners_religious_commitment = match field {
        "the same as mine" => PartnersReligionResponse::Same,
        "within one level of mine" => PartnersReligionResponse::Within1Level,
        "it doesn't matter" => PartnersReligionResponse::DoesNotMatter,
        other => bail!(
            "Unexpected response for partners religious commitment \"{}\", expected \"the same as mine\" or \"within one level of mine\" or \"it doesn't matter\"",
            other
        ),
    };

    Ok(Dealbreakers {
        wants_children,
        marriage_timeline,
        stay_local,
        my_religious_commitment,
        partners_religious_commitment,
    })
}

fn parse_corevalues<'i>(
    header_and_field: &mut impl Iterator<Item = (&'i str, &'i str)>,
) -> Result<CoreValues> {
    let response_and_weights = header_and_field
        .by_ref()
        .tuples::<(_, _)>()
        .take(14)
        .map(|((_, q_value), (w_header, w_value))| {
            ensure!(w_header == "From the question above, how important is it that your partner feels the same way about this as you do?", "Unexpected header \"{}\", expected \"From the question above, how important is it that your partner feels the same way about this as you do?\"", w_header);

            let response = FourChoiceResponse::new(q_value)?;
            let weight = FiveChoiceWeight::new(w_value)?;
        Ok(ResponseAndWeight { response, weight })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(CoreValues {
        response_and_weights: response_and_weights
            .try_into()
            .map_err(|e| anyhow::anyhow!("Conversion of vec to array failed: {:?}", e))?,
    })
}

fn parse_relationshipdynamics<'i>(
    header_and_field: &mut impl Iterator<Item = (&'i str, &'i str)>,
) -> Result<RelationshipDynamics> {
    let response_and_weights = header_and_field
        .by_ref()
        .tuples::<(_, _)>()
        .take(8)
        .map(|((_, q_value), (w_header, w_value))| {
            ensure!(w_header == "From the question above, how important is it that your partner feels the same way about this as you do?", "Unexpected header \"{}\", expected \"From the question above, how important is it that your partner feels the same way about this as you do?\"", w_header);

            let response = FourChoiceResponse::new(q_value)?;
            let weight = FiveChoiceWeight::new(w_value)?;
        Ok(ResponseAndWeight { response, weight })
        })
        .collect::<Result<Vec<_>>>()?;

    let responses = header_and_field
        .by_ref()
        .take(3)
        .map(|(_, q_value)| {
            let response = FourChoiceResponse::new(q_value)?;
            Ok(response)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(RelationshipDynamics {
        response_and_weights: response_and_weights
            .try_into()
            .map_err(|e| anyhow::anyhow!("Conversion of vec to array failed: {:?}", e))?,
        responses: responses
            .try_into()
            .map_err(|e| anyhow::anyhow!("Conversion of vec to array failed: {:?}", e))?,
    })
}

fn parse_lifestylemoney<'i>(
    header_and_field: &mut impl Iterator<Item = (&'i str, &'i str)>,
) -> Result<LifestyleMoney> {
    let responses = header_and_field
        .by_ref()
        .take(8)
        .map(|(_, q_value)| {
            let response = FourChoiceResponse::new(q_value)?;
            Ok(response)
        })
        .collect::<Result<Vec<_>>>()?;

    let (header, field) = header_and_field
        .next()
        .ok_or_else(|| anyhow!("Unexpected reached the end of csv record"))?;
    ensure!(
        header == "I'd like to have ___ child (children)",
        "Unexpected header \"{}\", expected \"I'd like to have ___ child (children)\"",
        header
    );

    let num_children = NumChildren::new(field)?;

    Ok(LifestyleMoney {
        responses: responses
            .try_into()
            .map_err(|e| anyhow::anyhow!("Conversion of vec to array failed: {:?}", e))?,
        num_children,
    })
}

fn parse_selfdescription<'i>(
    header_and_field: &mut impl Iterator<Item = (&'i str, &'i str)>,
) -> Result<SelfDescription> {
    let responses = header_and_field
        .by_ref()
        .take(15)
        .map(|(_, q_value)| {
            let response = FourChoiceResponse::new(q_value)?;
            Ok(response)
        })
        .collect::<Result<Vec<_>>>()?;

    let crossmatched_indices = [0, 1, 2, 3, 7, 8, 10, 11];
    let crossmatched: [FourChoiceResponse; 8] = crossmatched_indices
        .iter()
        .map(|&i| responses[i].clone())
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|e| anyhow::anyhow!("Conversion of vec to array failed: {:?}", e))?;

    let direct_indices = [4, 5, 6, 9, 12, 13, 14];
    let direct: [FourChoiceResponse; 7] = direct_indices
        .iter()
        .map(|&i| responses[i].clone())
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|e| anyhow::anyhow!("Conversion of vec to array failed: {:?}", e))?;
    Ok(SelfDescription {
        crossmatched,
        direct,
    })
}

fn parse_partnerpreferences<'i>(
    header_and_field: &mut impl Iterator<Item = (&'i str, &'i str)>,
) -> Result<PartnerPreferences> {
    let responses = header_and_field
        .by_ref()
        .take(10)
        .map(|(_, q_value)| {
            let response = FourChoiceResponse::new(q_value)?;
            Ok(response)
        })
        .collect::<Result<Vec<_>>>()?;

    let crossmatched_indices = [0, 1, 2, 3, 4, 5, 6, 7];
    let crossmatched: [FourChoiceResponse; 8] = crossmatched_indices
        .iter()
        .map(|&i| responses[i].clone())
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|e| anyhow::anyhow!("Conversion of vec to array failed: {:?}", e))?;

    let direct_indices = [8, 9];
    let direct: [FourChoiceResponse; 2] = direct_indices
        .iter()
        .map(|&i| responses[i].clone())
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|e| anyhow::anyhow!("Conversion of vec to array failed: {:?}", e))?;
    Ok(PartnerPreferences {
        crossmatched,
        direct,
    })
}

fn parse_socialstyle<'i>(
    header_and_field: &mut impl Iterator<Item = (&'i str, &'i str)>,
) -> Result<SocialStyle> {
    let responses = header_and_field
        .by_ref()
        .take(8)
        .map(|(_, q_value)| {
            let response = FourChoiceResponse::new(q_value)?;
            Ok(response)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(SocialStyle {
        responses: responses
            .try_into()
            .map_err(|e| anyhow::anyhow!("Conversion of vec to array failed: {:?}", e))?,
    })
}

fn parse_interests<'i>(
    header_and_field: &mut impl Iterator<Item = (&'i str, &'i str)>,
) -> Result<Interests> {
    let responses = header_and_field
        .by_ref()
        .take(8)
        .map(|(_, q_value)| {
            let response = FourChoiceResponse::new(q_value)?;
            Ok(response)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Interests {
        responses: responses
            .try_into()
            .map_err(|e| anyhow::anyhow!("Conversion of vec to array failed: {:?}", e))?,
    })
}

fn parse_freeresponse<'i>(
    header_and_field: &mut impl Iterator<Item = (&'i str, &'i str)>,
) -> Result<FreeResponse> {
    let responses = header_and_field
        .by_ref()
        .take(8)
        .filter_map(|(q_header, q_value)| {
            if q_value.is_empty() {
                None
            } else {
                Some((q_header.to_string(), q_value.to_string()))
            }
        })
        .collect::<HashMap<String, String>>();

    Ok(FreeResponse { responses })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_parse() {
        let data = r#"Timestamp,Email Address,First and last name,Gender,Age,I want to have children,I'd like to be married within,I intend to stay in Cache Valley long term,My religious commitment level,My partner's religious commitment level should be:,Protecting feelings matters more than blunt honesty,"From the question above, how important is it that your partner feels the same way about this as you do?",Social activism is important to me,"From the question above, how important is it that your partner feels the same way about this as you do?",There is a place for revenge when someone wrongs you,"From the question above, how important is it that your partner feels the same way about this as you do?",Some things are simply black and white,"From the question above, how important is it that your partner feels the same way about this as you do?","The phrase ""I love you"" is a promise","From the question above, how important is it that your partner feels the same way about this as you do?",I go to great lengths to minimize harm to the planet,"From the question above, how important is it that your partner feels the same way about this as you do?",I would keep a gun in the house,"From the question above, how important is it that your partner feels the same way about this as you do?",I would end a friendship over political differences,"From the question above, how important is it that your partner feels the same way about this as you do?",No one can be truly self-made,"From the question above, how important is it that your partner feels the same way about this as you do?",Everyone deserves my empathy,"From the question above, how important is it that your partner feels the same way about this as you do?",I would rather fail than cheat,"From the question above, how important is it that your partner feels the same way about this as you do?",I am the most important person in my own life,"From the question above, how important is it that your partner feels the same way about this as you do?",I prefer politically incorrect humor,"From the question above, how important is it that your partner feels the same way about this as you do?",Buying local over corporate matters to me,"From the question above, how important is it that your partner feels the same way about this as you do?",I say what's bothering me even if it makes my partner uncomfortable,"From the question above, how important is it that your partner feels the same way about this as you do?",I can't sleep if my partner is upset with me,"From the question above, how important is it that your partner feels the same way about this as you do?",My partner can be just friends with an ex,"From the question above, how important is it that your partner feels the same way about this as you do?",I'd want my partner to share their location with me,"From the question above, how important is it that your partner feels the same way about this as you do?",My partner should enjoy spending time with my family without me,"From the question above, how important is it that your partner feels the same way about this as you do?",My parents' approval of my partner matters to me,"From the question above, how important is it that your partner feels the same way about this as you do?",I run major decisions by my parents,"From the question above, how important is it that your partner feels the same way about this as you do?",I'd rather ghost than directly reject someone,"From the question above, how important is it that your partner feels the same way about this as you do?",I avoid burning bridges at all costs,I check in on friends regularly,I need friends who respond quickly to messages,Expensive dates are more fun,I count every penny I spend,I like to indulge in non-essential purchases,Financial stability matters more than chasing passion,It matters that I earn more than my peers,I want an extravagant wedding,My kids should attend private school,I enjoy treating myself even when impractical,I'd like to have ___ child (children),I tend to plan things carefully,I have an artistic or creative side,I am energetic and outgoing,I am highly goal-oriented and driven,I prefer structured daily routines,Idle days with no plans feel,I usually find it harder to,I have a dry sense of humor,I enjoy intellectual debate and sparring,My profession is a defining part of who I am,Diet and nutrition are important to me,Staying active and exercising matters,Fashion and personal style matter to me,Sports are an important part of my identity,I'm comfortable being spontaneous over responsible,I'd prefer a partner who plans vs. goes with the flow,An artistic or creative side in a partner matters,I'd prefer a partner who is,Ambition in a partner matters to me,A dry sense of humor in a partner matters,An intellectually curious partner matters to me,A health-conscious partner matters to me,An active or fit partner matters to me,Splitting the bill on a first date feels right,I'm fine with my partner having celebrity crushes,I prefer a small close-knit friend group,I enjoy social media and actively engage with it,I love planning and hosting gatherings,I value deep conversations over casual small talk,I enjoy playful teasing with friends,I'm comfortable with friends who hold different beliefs,"I enjoy group activities (sports, games, trivia)",I prefer cozy nights in over going out,"I enjoy philosophy, science, or psychology discussions","I follow pop culture, TV, and movies","I enjoy concerts, festivals, and live events","I enjoy thrill-seeking (skydiving, roller coasters)",I could spend hours lost in a book or creative project,I enjoy dark humor and sarcasm,I love trying exotic or unusual foods,I enjoy discussing current events,Unpopular opinion I stand by:,Something I've changed my mind about recently:,I could give a 10-minute talk on:,Ideal low-effort hangout:,My weekend usually looks like:,Niche interest most people don't know I have:,Something I'm better at than I let on:,The thing I find most attractive in a person:
           4/11/2026 22:43:04,ephraimkunz@me.com,Ephraim kunz,Male,37,No,2 - 5 years,No,1,the same as mine,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,1,1,1,1,1,1,1,4,1,1,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,rgerg,hjkhjkhj,ewfwef,,dfhfdh,qwewefwef,dfgfdgdfg,jkkjy"#;

        let mut reader = csv::Reader::from_reader(data.as_bytes());
        let result = parse_responses(&mut reader).unwrap();
        assert!(result.len() == 1);
    }

    #[test]
    fn too_many_columns() {
        let data = r#"yellow,Timestamp,Email Address,First and last name,Gender,Age,I want to have children,I'd like to be married within,I intend to stay in Cache Valley long term,My religious commitment level,My partner's religious commitment level should be:,Protecting feelings matters more than blunt honesty,"From the question above, how important is it that your partner feels the same way about this as you do?",Social activism is important to me,"From the question above, how important is it that your partner feels the same way about this as you do?",There is a place for revenge when someone wrongs you,"From the question above, how important is it that your partner feels the same way about this as you do?",Some things are simply black and white,"From the question above, how important is it that your partner feels the same way about this as you do?","The phrase ""I love you"" is a promise","From the question above, how important is it that your partner feels the same way about this as you do?",I go to great lengths to minimize harm to the planet,"From the question above, how important is it that your partner feels the same way about this as you do?",I would keep a gun in the house,"From the question above, how important is it that your partner feels the same way about this as you do?",I would end a friendship over political differences,"From the question above, how important is it that your partner feels the same way about this as you do?",No one can be truly self-made,"From the question above, how important is it that your partner feels the same way about this as you do?",Everyone deserves my empathy,"From the question above, how important is it that your partner feels the same way about this as you do?",I would rather fail than cheat,"From the question above, how important is it that your partner feels the same way about this as you do?",I am the most important person in my own life,"From the question above, how important is it that your partner feels the same way about this as you do?",I prefer politically incorrect humor,"From the question above, how important is it that your partner feels the same way about this as you do?",Buying local over corporate matters to me,"From the question above, how important is it that your partner feels the same way about this as you do?",I say what's bothering me even if it makes my partner uncomfortable,"From the question above, how important is it that your partner feels the same way about this as you do?",I can't sleep if my partner is upset with me,"From the question above, how important is it that your partner feels the same way about this as you do?",My partner can be just friends with an ex,"From the question above, how important is it that your partner feels the same way about this as you do?",I'd want my partner to share their location with me,"From the question above, how important is it that your partner feels the same way about this as you do?",My partner should enjoy spending time with my family without me,"From the question above, how important is it that your partner feels the same way about this as you do?",My parents' approval of my partner matters to me,"From the question above, how important is it that your partner feels the same way about this as you do?",I run major decisions by my parents,"From the question above, how important is it that your partner feels the same way about this as you do?",I'd rather ghost than directly reject someone,"From the question above, how important is it that your partner feels the same way about this as you do?",I avoid burning bridges at all costs,I check in on friends regularly,I need friends who respond quickly to messages,Expensive dates are more fun,I count every penny I spend,I like to indulge in non-essential purchases,Financial stability matters more than chasing passion,It matters that I earn more than my peers,I want an extravagant wedding,My kids should attend private school,I enjoy treating myself even when impractical,I'd like to have ___ child (children),I tend to plan things carefully,I have an artistic or creative side,I am energetic and outgoing,I am highly goal-oriented and driven,I prefer structured daily routines,Idle days with no plans feel,I usually find it harder to,I have a dry sense of humor,I enjoy intellectual debate and sparring,My profession is a defining part of who I am,Diet and nutrition are important to me,Staying active and exercising matters,Fashion and personal style matter to me,Sports are an important part of my identity,I'm comfortable being spontaneous over responsible,I'd prefer a partner who plans vs. goes with the flow,An artistic or creative side in a partner matters,I'd prefer a partner who is,Ambition in a partner matters to me,A dry sense of humor in a partner matters,An intellectually curious partner matters to me,A health-conscious partner matters to me,An active or fit partner matters to me,Splitting the bill on a first date feels right,I'm fine with my partner having celebrity crushes,I prefer a small close-knit friend group,I enjoy social media and actively engage with it,I love planning and hosting gatherings,I value deep conversations over casual small talk,I enjoy playful teasing with friends,I'm comfortable with friends who hold different beliefs,"I enjoy group activities (sports, games, trivia)",I prefer cozy nights in over going out,"I enjoy philosophy, science, or psychology discussions","I follow pop culture, TV, and movies","I enjoy concerts, festivals, and live events","I enjoy thrill-seeking (skydiving, roller coasters)",I could spend hours lost in a book or creative project,I enjoy dark humor and sarcasm,I love trying exotic or unusual foods,I enjoy discussing current events,Unpopular opinion I stand by:,Something I've changed my mind about recently:,I could give a 10-minute talk on:,Ideal low-effort hangout:,My weekend usually looks like:,Niche interest most people don't know I have:,Something I'm better at than I let on:,The thing I find most attractive in a person:
           hi,4/11/2026 22:43:04,ephraimkunz@me.com,Ephraim kunz,Male,37,No,2 - 5 years,No,1,the same as mine,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,1,1,1,1,1,1,1,4,1,1,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,rgerg,hjkhjkhj,ewfwef,,dfhfdh,qwewefwef,dfgfdgdfg,jkkjy"#;

        let mut reader = csv::Reader::from_reader(data.as_bytes());
        let result = parse_responses(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn not_enough_columns() {
        let data = r#"Email Address,First and last name,Gender,Age,I want to have children,I'd like to be married within,I intend to stay in Cache Valley long term,My religious commitment level,My partner's religious commitment level should be:,Protecting feelings matters more than blunt honesty,"From the question above, how important is it that your partner feels the same way about this as you do?",Social activism is important to me,"From the question above, how important is it that your partner feels the same way about this as you do?",There is a place for revenge when someone wrongs you,"From the question above, how important is it that your partner feels the same way about this as you do?",Some things are simply black and white,"From the question above, how important is it that your partner feels the same way about this as you do?","The phrase ""I love you"" is a promise","From the question above, how important is it that your partner feels the same way about this as you do?",I go to great lengths to minimize harm to the planet,"From the question above, how important is it that your partner feels the same way about this as you do?",I would keep a gun in the house,"From the question above, how important is it that your partner feels the same way about this as you do?",I would end a friendship over political differences,"From the question above, how important is it that your partner feels the same way about this as you do?",No one can be truly self-made,"From the question above, how important is it that your partner feels the same way about this as you do?",Everyone deserves my empathy,"From the question above, how important is it that your partner feels the same way about this as you do?",I would rather fail than cheat,"From the question above, how important is it that your partner feels the same way about this as you do?",I am the most important person in my own life,"From the question above, how important is it that your partner feels the same way about this as you do?",I prefer politically incorrect humor,"From the question above, how important is it that your partner feels the same way about this as you do?",Buying local over corporate matters to me,"From the question above, how important is it that your partner feels the same way about this as you do?",I say what's bothering me even if it makes my partner uncomfortable,"From the question above, how important is it that your partner feels the same way about this as you do?",I can't sleep if my partner is upset with me,"From the question above, how important is it that your partner feels the same way about this as you do?",My partner can be just friends with an ex,"From the question above, how important is it that your partner feels the same way about this as you do?",I'd want my partner to share their location with me,"From the question above, how important is it that your partner feels the same way about this as you do?",My partner should enjoy spending time with my family without me,"From the question above, how important is it that your partner feels the same way about this as you do?",My parents' approval of my partner matters to me,"From the question above, how important is it that your partner feels the same way about this as you do?",I run major decisions by my parents,"From the question above, how important is it that your partner feels the same way about this as you do?",I'd rather ghost than directly reject someone,"From the question above, how important is it that your partner feels the same way about this as you do?",I avoid burning bridges at all costs,I check in on friends regularly,I need friends who respond quickly to messages,Expensive dates are more fun,I count every penny I spend,I like to indulge in non-essential purchases,Financial stability matters more than chasing passion,It matters that I earn more than my peers,I want an extravagant wedding,My kids should attend private school,I enjoy treating myself even when impractical,I'd like to have ___ child (children),I tend to plan things carefully,I have an artistic or creative side,I am energetic and outgoing,I am highly goal-oriented and driven,I prefer structured daily routines,Idle days with no plans feel,I usually find it harder to,I have a dry sense of humor,I enjoy intellectual debate and sparring,My profession is a defining part of who I am,Diet and nutrition are important to me,Staying active and exercising matters,Fashion and personal style matter to me,Sports are an important part of my identity,I'm comfortable being spontaneous over responsible,I'd prefer a partner who plans vs. goes with the flow,An artistic or creative side in a partner matters,I'd prefer a partner who is,Ambition in a partner matters to me,A dry sense of humor in a partner matters,An intellectually curious partner matters to me,A health-conscious partner matters to me,An active or fit partner matters to me,Splitting the bill on a first date feels right,I'm fine with my partner having celebrity crushes,I prefer a small close-knit friend group,I enjoy social media and actively engage with it,I love planning and hosting gatherings,I value deep conversations over casual small talk,I enjoy playful teasing with friends,I'm comfortable with friends who hold different beliefs,"I enjoy group activities (sports, games, trivia)",I prefer cozy nights in over going out,"I enjoy philosophy, science, or psychology discussions","I follow pop culture, TV, and movies","I enjoy concerts, festivals, and live events","I enjoy thrill-seeking (skydiving, roller coasters)",I could spend hours lost in a book or creative project,I enjoy dark humor and sarcasm,I love trying exotic or unusual foods,I enjoy discussing current events,Unpopular opinion I stand by:,Something I've changed my mind about recently:,I could give a 10-minute talk on:,Ideal low-effort hangout:,My weekend usually looks like:,Niche interest most people don't know I have:,Something I'm better at than I let on:,The thing I find most attractive in a person:
           ephraimkunz@me.com,Ephraim kunz,Male,37,No,2 - 5 years,No,1,the same as mine,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,I don't care if we agree,1,1,1,1,1,1,1,1,4,1,1,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,rgerg,hjkhjkhj,ewfwef,,dfhfdh,qwewefwef,dfgfdgdfg,jkkjy"#;

        let mut reader = csv::Reader::from_reader(data.as_bytes());
        let result = parse_responses(&mut reader);
        assert!(result.is_err());
    }

    #[test]
    fn weight_normalization() {
        assert!(FiveChoiceWeight(0).normalized() == 0.25);
        assert!(FiveChoiceWeight(1).normalized() == 0.6875);
        assert!(FiveChoiceWeight(2).normalized() == 1.125);
        assert!(FiveChoiceWeight(3).normalized() == 1.5625);
        assert!(FiveChoiceWeight(4).normalized() == 2.0);
    }
}

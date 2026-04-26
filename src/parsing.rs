use anyhow::{Context, Result, anyhow, bail, ensure};
use itertools::Itertools;
use std::{collections::HashMap, path::Path, vec};

#[derive(Clone, Debug)]
pub struct QuestionnaireResponse {
    demographics: Demographics,
    dealbreakers: Dealbreakers,
    corevalues: CoreValues,
    relationshipdynamics: RelationshipDynamics,
    lifestylemoney: LifestyleMoney,
    selfdescription: SelfDescription,
    partnerpreferences: PartnerPreferences,
    socialstyle: SocialStyle,
    interests: Interests,
    freeresponse: FreeResponse,
}

#[derive(Clone, Debug)]
pub struct Demographics {
    name: String,
    email: String,
    gender: Gender,
    age: u8,
}

#[derive(Clone, Debug)]
pub struct Dealbreakers {
    wants_children: YesNoMaybeResponse,
    marriage_timeline: MarriageTimelineResponse,
    stay_local: YesNoMaybeResponse,
    my_religious_commitment: u8,
    partners_religious_commitment: PartnersReligionResponse,
}

#[derive(Clone, Debug)]
pub struct CoreValues {
    response_and_weights: [ResponseAndWeight; 14],
}

#[derive(Clone, Debug)]
pub struct RelationshipDynamics {
    response_and_weights: [ResponseAndWeight; 8],
    responses: [FourChoiceResponse; 3],
}

#[derive(Clone, Debug)]
pub struct LifestyleMoney {
    responses: [FourChoiceResponse; 8],
    num_children: u8,
}

#[derive(Clone, Debug)]
pub struct SelfDescription {
    /// Crossmatched against corresponding item in PartnerPreferences.crossmatched
    crossmatched: [FourChoiceResponse; 8],

    /// Direct comparison with corresponding SelfDescription.direct for partner candidate.
    direct: [FourChoiceResponse; 7],
}

#[derive(Clone, Debug)]
pub struct PartnerPreferences {
    /// Crossmatched against corresponding item in SelfDescription.crossmatched
    crossmatched: [FourChoiceResponse; 8],

    /// Direct comparison with corresponding PartnerPreferences.direct for partner candidate.
    direct: [FourChoiceResponse; 2],
}

#[derive(Clone, Debug)]
pub struct SocialStyle {
    responses: [FourChoiceResponse; 8],
}

#[derive(Clone, Debug)]
pub struct Interests {
    responses: [FourChoiceResponse; 8],
}

#[derive(Clone, Debug)]
pub struct FreeResponse {
    responses: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct FourChoiceResponse(u8);

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
}

#[derive(Clone, Debug)]
pub struct ResponseAndWeight {
    response: FourChoiceResponse,
    weight: FiveChoiceWeight,
}

#[derive(Clone, Debug)]
pub struct FiveChoiceWeight(u8);

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
}

#[derive(Clone, Debug)]
pub enum Gender {
    Male,
    Female,
}

#[derive(Clone, Debug)]
pub enum YesNoMaybeResponse {
    Yes,
    No,
    Maybe,
}

#[derive(Clone, Debug)]
pub enum MarriageTimelineResponse {
    ZeroToTwo,
    TwoToFive,
    FivePlus,
}

#[derive(Clone, Debug)]
pub enum PartnersReligionResponse {
    Same,
    Within1Level,
    DoesNotMatter,
}

pub fn parse_responses<P: AsRef<Path>>(path: P) -> Result<Vec<QuestionnaireResponse>> {
    let mut reader = csv::Reader::from_path(path)?;
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

    dbg!(&responses);
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
    let age: u8 = field.parse().context("Unable to parse age to number")?;
    ensure!(
        (26..=37).contains(&age),
        "Age {age} is not in the correct range"
    );

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
    let my_religious_commitment: u8 = field
        .parse()
        .context("Unable to parse my religious commitment to number")?;
    ensure!(
        (1..=5).contains(&my_religious_commitment),
        "My religious commitment {my_religious_commitment} is not in the correct range"
    );

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

    let num_children: u8 = field
        .parse()
        .context("Unable to parse num children to number")?;
    ensure!(
        (0..=9).contains(&num_children),
        "Num children {num_children} is not in the correct range"
    );

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

    Ok(FreeResponse {
        responses: responses,
    })
}

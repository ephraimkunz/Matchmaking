use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Result;
use itertools::Itertools;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{Matches, matching::MatchCard, parsing::Gender, validation::validated_name};

// Custom function to parse all-caps or standard booleans
fn deserialize_uppercase_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    // First, deserialize into a standard String
    let s = String::deserialize(deserializer)?;

    // Match against uppercase and standard variations
    match s.to_uppercase().as_str() {
        "TRUE" | "true" | "t" | "yes" | "y" => Ok(true),
        "FALSE" | "false" | "f" | "no" | "n" => Ok(false),
        _ => Err(serde::de::Error::custom(format!(
            "expected TRUE or FALSE, found: {s}",
        ))),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AttendanceRow {
    name: String,
    gender: Gender,
    #[serde(deserialize_with = "deserialize_uppercase_bool")]
    attending: bool,
}

#[derive(Debug)]
pub struct Schedule {
    pub stderr: String,
    pub stdout: String,
}

#[derive(Serialize)]
struct ModelData {
    /// How many people participating
    #[serde(rename = "N")]
    n: usize,

    /// Number of rounds
    #[serde(rename = "R")]
    r: usize,

    /// Matrix of who wants who
    wants: Vec<Vec<bool>>,

    /// Names of each participant in the same order as wants
    names: Vec<String>,

    /// True if participant is male, in the same order as wants
    is_male: Vec<bool>,

    /// True if participant is attending, in the same order as wants
    attending: Vec<bool>,
}

/// # Errors
///
/// Returns an error when unable to open or write the output CSV file.
pub fn generate_attendance(matches: &Matches) -> Result<PathBuf> {
    let attendance_path = Path::new("./attendance.csv").to_path_buf();
    let mut writer = csv::Writer::from_path(&attendance_path)?;
    let records = matches.0.iter().map(|m| AttendanceRow {
        name: m.name.clone(),
        gender: m.gender.clone(),
        attending: false,
    });

    for record in records {
        writer.serialize(record)?;
    }

    Ok(attendance_path)
}

/// # Errors
///
/// Returns an error when there is no schedule due to constraint solving failing.
pub fn generate_schedule(matches: &Matches, attendance_path: Option<&Path>) -> Result<Schedule> {
    let matches = matches_and_attendance(matches, attendance_path)?;

    let r = 3;
    let n = matches.len();

    let mut wants = vec![vec![false; n]; n];
    let name_to_id: FxHashMap<&str, usize> = matches
        .iter()
        .enumerate()
        .map(|(id, m)| (m.0.name.as_str(), id))
        .collect();
    for m in &matches {
        let p = name_to_id[m.0.name.as_str()];

        for s in &m.0.shortlist {
            let Some(&q) = name_to_id.get(s.name.as_str()) else {
                anyhow::bail!("Unknown person in shortlist: {}", s.email);
            };
            wants[p][q] = true;
        }
    }

    let names = matches.iter().map(|m| m.0.name.clone()).collect_vec();
    let attending = matches.iter().map(|m| m.1).collect_vec();
    let is_male = matches
        .iter()
        .map(|m| m.0.gender == Gender::Male)
        .collect_vec();

    let data = ModelData {
        n,
        r,
        wants,
        names,
        is_male,
        attending,
    };

    let data_path = Path::new("./scheduling_data.json").to_path_buf();
    let data_file = std::fs::File::create(&data_path)?;
    serde_json::to_writer(data_file, &data)?;

    let output = Command::new("minizinc")
        .arg("--solver")
        .arg("cp-sat")
        .arg("-v")
        .arg("-p")
        .arg("10")
        .arg("./constraints.mzn")
        .arg(&data_path)
        .output()?;

    std::fs::remove_file(data_path)?;

    Ok(Schedule {
        stderr: String::from_utf8(output.stderr)?,
        stdout: String::from_utf8(output.stdout)?,
    })
}

fn matches_and_attendance(
    matches: &Matches,
    attendance_path: Option<&Path>,
) -> Result<Vec<(MatchCard, bool)>> {
    let Some(attendance_path) = attendance_path else {
        return Ok(matches.0.iter().map(|m| (m.clone(), true)).collect());
    };

    let mut reader = csv::Reader::from_path(attendance_path)?;
    let attendance_rows: Result<Vec<AttendanceRow>> = reader
        .deserialize()
        .map(|r| r.map_err(std::convert::Into::into))
        .collect();

    let mut attending: FxHashMap<String, Gender> = FxHashMap::default();

    for row in attendance_rows? {
        if row.attending {
            let name = validated_name(&row.name)?;
            attending.insert(name, row.gender);
        }
    }

    let mut result = vec![];
    for m in &matches.0 {
        if attending.contains_key(&m.name) {
            attending.remove(&m.name);
            result.push((m.clone(), true));
        } else {
            result.push((m.clone(), false));
        }
    }

    for (name, gender) in attending {
        result.push((
            MatchCard {
                name,
                email: String::new(),
                gender,
                shortlist: vec![],
            },
            true,
        ));
    }

    Ok(result)
}

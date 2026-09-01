use std::{path::Path, process::Command};

use anyhow::Result;
use itertools::Itertools;
use rustc_hash::FxHashMap;
use serde::Serialize;

use crate::Matches;

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
}

/// # Errors
///
/// Returns and error when there is no schedule due to constraint solving failing.
pub fn generate_schedule(matches: &Matches) -> Result<Schedule> {
    let r = 3;
    let n = matches.0.len();

    let mut wants = vec![vec![false; n]; n];
    let email_to_id: FxHashMap<&str, usize> = matches
        .0
        .iter()
        .enumerate()
        .map(|(id, m)| (m.email.as_str(), id))
        .collect();
    for m in &matches.0 {
        let p = email_to_id[m.email.as_str()];

        for s in &m.shortlist {
            let Some(&q) = email_to_id.get(s.email.as_str()) else {
                anyhow::bail!("Unknown person in shortlist: {}", s.email);
            };
            wants[p][q] = true;
        }
    }

    let names = matches.0.iter().map(|m| m.name.clone()).collect_vec();

    let data = ModelData { n, r, wants, names };

    let data_path = Path::new("./scheduling_data.json").to_path_buf();
    let data_file = std::fs::File::create(&data_path)?;
    serde_json::to_writer(data_file, &data)?;

    let output = Command::new("minizinc")
        .arg("--solver")
        .arg("cp-sat")
        // .arg("-v")
        .arg("-p")
        .arg("8")
        .arg("./constraints.mzn")
        .arg(&data_path)
        .output()?;

    std::fs::remove_file(data_path)?;

    Ok(Schedule {
        stderr: String::from_utf8(output.stderr)?,
        stdout: String::from_utf8(output.stdout)?,
    })
}

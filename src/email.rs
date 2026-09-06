use std::{io::Write, path::Path};

use anyhow::{Context, Result};
use english_numbers::Formatting;

use crate::Matches;

/// # Errors
///
/// Returns errors if the template can't be found or loaded, or if there are issues writing to stdout.
pub fn generate_email<W: Write>(
    matches: &Matches,
    template_path: &Path,
    total_match_count: usize,
    stdout: &mut W,
) -> Result<()> {
    let template = std::fs::read_to_string(template_path)
        .with_context(|| "Unable to read template_path into string")?;

    for card in &matches.0 {
        let email_address = card.email.as_str();
        let template = template.replace(
            "{{name}}",
            card.name.split_whitespace().next().unwrap_or(&card.name),
        );

        let template = template.replace("{{total_match_count}}", &format!("{total_match_count}"));

        let template = template.replace(
            "{{personal_match_count_title}}",
            &formatted_number(card.shortlist.len(), true),
        );

        let template = template.replace(
            "{{personal_match_count_body}}",
            &formatted_number(card.shortlist.len(), false),
        );

        let mut shortlist_bytes = vec![];
        for (index, m) in card.shortlist.iter().enumerate() {
            writeln!(shortlist_bytes, "{}", m.plaintext(false)?)?;

            if index < (card.shortlist.len() - 1) {
                writeln!(shortlist_bytes)?;
            }
        }
        let shortlist = String::from_utf8(shortlist_bytes)?;
        let template = template.replace("{{shortlist}}", &shortlist);

        write!(stdout, "{email_address}\n\n\n{template}\n\n\n\n\n\n")?;
    }

    Ok(())
}

fn formatted_number(n: usize, capitalized: bool) -> String {
    // Numbers 0-9 spelled out, 10+ digits only
    if let Ok(n) = i64::try_from(n)
        && n < 10
    {
        let mut formatting = Formatting::all();
        formatting.title_case = capitalized;
        english_numbers::convert(n, formatting)
    } else {
        format!("{n}")
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn basic() {
        let mut stdout = vec![];
        assert!(
            generate_email(
                &Matches(vec![]),
                Path::new("./test_data/test_email_template.txt"),
                0,
                &mut stdout
            )
            .is_ok()
        );

        let string = String::from_utf8(stdout).unwrap();
        assert!(!string.contains("{{name}}"));
        assert!(!string.contains("{{shortlist}}"));
    }
}

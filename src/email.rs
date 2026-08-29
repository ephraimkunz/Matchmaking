use std::{io::Write, path::PathBuf};

use anyhow::{Context, Result, anyhow};

use crate::Matches;

/// # Errors
///
/// Returns errors if the template can't be found or loaded, or if there are issues writing to stdout.
pub fn generate_email<W: Write>(
    matches: &Matches,
    template_path: Option<PathBuf>,
    stdout: &mut W,
) -> Result<()> {
    // template_path was validated as always being present if we got here in Args::validate.
    let template_path = template_path.ok_or(anyhow!(
        "Template_path was null in generate_email, which should be impossible"
    ))?;
    let template = std::fs::read_to_string(template_path)
        .with_context(|| "Unable to read template_path into string")?;

    for card in &matches.0 {
        let email_address = card.email.as_str();
        let template = template.replace(
            "{{name}}",
            card.name.split_whitespace().next().unwrap_or(&card.name),
        );

        let template = template.replace("{{total_match_count}}", &format!("{}", matches.0.len()));

        let template = template.replace(
            "{{personal_match_count}}",
            &format!("{}", card.shortlist.len()),
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
                Some(Path::new("./test_data/test_email_template.txt").to_path_buf()),
                &mut stdout
            )
            .is_ok()
        );

        let string = String::from_utf8(stdout).unwrap();
        assert!(!string.contains("{{name}}"));
        assert!(!string.contains("{{shortlist}}"));
    }
}

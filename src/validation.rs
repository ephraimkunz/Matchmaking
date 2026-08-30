use anyhow::{Result, ensure};
use itertools::Itertools;
use regex::Regex;
use rustc_hash::FxHashSet;
use std::sync::LazyLock;

static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .expect("Invalid email regex pattern")
});

pub fn validated_email(email: &str) -> Result<String> {
    let trimmed = email.trim();

    ensure!(!trimmed.is_empty(), "Email address is whitespace or empty");
    ensure!(
        EMAIL_REGEX.is_match(trimmed),
        "{trimmed} is not a valid email"
    );

    Ok(trimmed.to_lowercase())
}

pub fn validated_name(name: &str) -> Result<String> {
    let trimmed = name.trim();

    ensure!(!trimmed.is_empty(), "Name is whitespace or empty");

    let components = name.split_whitespace().collect_vec();
    ensure!(
        (2..=3).contains(&components.len()),
        "Name \"{trimmed}\" should be 2-3 words long, was {}",
        components.len()
    );

    let capitalized = components.iter().map(|c| capitalize(c)).join(" ");

    Ok(capitalized)
}

pub fn validated_free_response(response: &str) -> Option<String> {
    let trimmed = response.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// # Errors
///
/// Returns an error if `ids` has members that are not valid emails or not in `allowed_ids`.
pub fn validate_ids<'a>(
    ids: &[String],
    allowed_ids: impl Iterator<Item = &'a str>,
) -> Result<FxHashSet<String>> {
    let ids: Result<Vec<_>> = ids.iter().map(|i| validated_email(i)).collect();
    let ids = ids?;
    let ids: FxHashSet<_> = ids.into_iter().collect();

    let mut ids_not_seen = ids.clone();
    for response_id in allowed_ids {
        ids_not_seen.remove(response_id);
    }

    ensure!(
        ids_not_seen.is_empty(),
        "Id list contains non-existent id(s): {}",
        ids_not_seen.iter().join(", ")
    );

    Ok(ids)
}

pub fn validate_id<'a>(id: &str, allowed_ids: impl Iterator<Item = &'a str>) -> Result<String> {
    let ids = validate_ids(&[id.to_string()], allowed_ids)?;
    Ok(ids.into_iter().next().unwrap())
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_emails() {
        assert!(validated_email("").is_err());
        assert!(validated_email(" ").is_err());
        assert!(validated_email("email").is_err());
        assert!(validated_email("email.c").is_err());
    }

    #[test]
    fn valid_emails() {
        assert_eq!(
            validated_email("ephraimkunz@example.com").unwrap(),
            "ephraimkunz@example.com".to_string()
        );
        assert_eq!(
            validated_email("ePhraimkunz@exampLe.com").unwrap(),
            "ephraimkunz@example.com".to_string()
        );
        assert_eq!(
            validated_email("  ephraimkunz@example.com ").unwrap(),
            "ephraimkunz@example.com".to_string()
        );
    }

    #[test]
    fn invalid_name() {
        assert!(validated_name("").is_err());
        assert!(validated_name(" ").is_err());
        assert!(validated_name("Ephraim").is_err());
        assert!(validated_name("This is a test").is_err());
    }

    #[test]
    fn valid_name() {
        assert_eq!(validated_name("ephraim kunz").unwrap(), "Ephraim Kunz");
        assert_eq!(
            validated_name("George edward   Santana").unwrap(),
            "George Edward Santana"
        );
        assert_eq!(validated_name("  Ephraim kunz ").unwrap(), "Ephraim Kunz");
    }

    #[test]
    fn invalid_free_response() {
        assert!(validated_free_response("").is_none());
        assert!(validated_free_response(" ").is_none());
    }

    #[test]
    fn valid_free_response() {
        assert_eq!(validated_free_response(" a ").unwrap(), "a");
        assert_eq!(
            validated_free_response("This is a test ").unwrap(),
            "This is a test"
        );
    }

    #[test]
    fn invalid_ids() {
        assert!(
            validate_ids(
                &["abc".to_string(), "def".to_string()],
                ["abc", "def"].iter().copied()
            )
            .is_err()
        );
    }

    #[test]
    fn in_sets_ids() {
        assert_eq!(
            validate_ids(
                &["B@example.com ".to_string(), "  a@example.com".to_string()],
                ["a@example.com", "b@example.com"].iter().copied()
            )
            .unwrap(),
            ["a@example.com".to_string(), "b@example.com".to_string()]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn not_in_sets_ids() {
        assert!(
            validate_ids(
                &["B@example.com ".to_string(), "  c@example.com".to_string()],
                ["a@example.com", "b@example.com"].iter().copied()
            )
            .is_err()
        );
    }
}

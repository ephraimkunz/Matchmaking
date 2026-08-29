use anyhow::{Result, ensure};
use itertools::Itertools;
use regex::Regex;
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

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use crate::parsing::validation::{validated_email, validated_free_response, validated_name};

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
}

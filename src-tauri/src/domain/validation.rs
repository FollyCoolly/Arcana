use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};
use std::fmt;

pub type ValidationResult = Result<(), ValidationErrors>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationIssue {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors {
    issues: Vec<ValidationIssue>,
}

impl ValidationErrors {
    pub fn new(issues: Vec<ValidationIssue>) -> Self {
        debug_assert!(!issues.is_empty());
        Self { issues }
    }

    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    pub fn into_issues(self) -> Vec<ValidationIssue> {
        self.issues
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, issue) in self.issues.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{} at {}: {}", issue.code, issue.path, issue.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

pub trait Validate {
    fn validate(&self) -> ValidationResult;
}

#[derive(Debug, Default)]
pub(crate) struct Validator {
    issues: Vec<ValidationIssue>,
}

impl Validator {
    pub(crate) fn error(
        &mut self,
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(ValidationIssue {
            code: code.into(),
            path: path.into(),
            message: message.into(),
        });
    }

    pub(crate) fn require(&mut self, condition: bool, code: &str, path: &str, message: &str) {
        if !condition {
            self.error(code, path, message);
        }
    }

    pub(crate) fn require_non_blank(&mut self, value: &str, path: &str) {
        self.require(
            !value.trim().is_empty(),
            "empty_string",
            path,
            "must not be empty or whitespace only",
        );
    }

    pub(crate) fn merge(&mut self, prefix: &str, result: ValidationResult) {
        let Err(errors) = result else {
            return;
        };
        for mut issue in errors.into_issues() {
            issue.path = if issue.path.is_empty() {
                prefix.to_string()
            } else {
                format!("{prefix}.{}", issue.path)
            };
            self.issues.push(issue);
        }
    }

    pub(crate) fn finish(self) -> ValidationResult {
        if self.issues.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors::new(self.issues))
        }
    }
}

pub(crate) fn is_valid_date(value: &str) -> bool {
    if value.len() != 10 {
        return false;
    }
    if !value
        .get(..4)
        .and_then(|year| year.parse::<i32>().ok())
        .is_some_and(|year| (1..=9999).contains(&year))
    {
        return false;
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

pub(crate) fn is_valid_partial_date(value: &str) -> bool {
    match value.len() {
        4 => value
            .parse::<i32>()
            .is_ok_and(|year| (1..=9999).contains(&year)),
        7 => is_valid_date(&format!("{value}-01")),
        10 => is_valid_date(value),
        _ => false,
    }
}

pub(crate) fn parse_rfc3339(value: &str) -> Option<DateTime<chrono::FixedOffset>> {
    DateTime::parse_from_rfc3339(value).ok()
}

pub(crate) fn is_sorted_unique<T, K: Ord + ?Sized>(
    items: &[T],
    key: impl for<'a> Fn(&'a T) -> &'a K,
) -> bool {
    items.windows(2).all(|pair| key(&pair[0]) < key(&pair[1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_validation_rejects_impossible_dates() {
        assert!(is_valid_date("2024-02-29"));
        assert!(!is_valid_date("2023-02-29"));
        assert!(!is_valid_date("2024-13-01"));
    }

    #[test]
    fn partial_dates_accept_only_supported_precision() {
        assert!(is_valid_partial_date("2024"));
        assert!(is_valid_partial_date("2024-02"));
        assert!(is_valid_partial_date("2024-02-29"));
        assert!(!is_valid_partial_date("2024-02-29T00:00:00Z"));
    }
}

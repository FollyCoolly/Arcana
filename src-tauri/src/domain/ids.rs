use regex::Regex;
use std::sync::OnceLock;

fn snake_case_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"^[a-z][a-z0-9_]*$").expect("valid ID regex"))
}

pub fn is_snake_case_id(value: &str) -> bool {
    snake_case_pattern().is_match(value)
}

pub fn split_record_definition_id(value: &str) -> Option<(&str, &str)> {
    let (namespace, name) = value.split_once('.')?;
    if name.contains('.') || !is_snake_case_id(namespace) || !is_snake_case_id(name) {
        return None;
    }
    Some((namespace, name))
}

pub fn split_scoped_id(value: &str) -> Option<(&str, &str)> {
    let (pack_id, local_id) = value.split_once("::")?;
    if local_id.contains("::") || !is_snake_case_id(pack_id) || !is_snake_case_id(local_id) {
        return None;
    }
    Some((pack_id, local_id))
}

pub fn is_portable_asset_path(value: &str) -> bool {
    if value.contains('\\') || !value.starts_with("assets/") {
        return false;
    }

    value.split('/').all(is_portable_segment)
}

fn is_portable_segment(segment: &str) -> bool {
    if segment.is_empty()
        || segment == "."
        || segment == ".."
        || segment.ends_with('.')
        || segment.ends_with(' ')
        || !segment.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
    {
        return false;
    }

    let stem = segment.split('.').next().unwrap_or(segment);
    !is_windows_reserved_name(stem)
}

fn is_windows_reserved_name(value: &str) -> bool {
    matches!(value, "con" | "prn" | "aux" | "nul")
        || (value.len() == 4
            && (value.starts_with("com") || value.starts_with("lpt"))
            && matches!(value.as_bytes()[3], b'1'..=b'9'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_definition_id_has_exactly_two_parts() {
        assert_eq!(
            split_record_definition_id("health.weight"),
            Some(("health", "weight"))
        );
        assert!(split_record_definition_id("health.weight.kg").is_none());
        assert!(split_record_definition_id("Health.weight").is_none());
    }

    #[test]
    fn scoped_id_uses_pack_and_local_snake_case() {
        assert_eq!(
            split_scoped_id("cooking::first_dish"),
            Some(("cooking", "first_dish"))
        );
        assert!(split_scoped_id("cooking:first_dish").is_none());
    }

    #[test]
    fn asset_path_is_cross_platform_safe() {
        assert!(is_portable_asset_path("assets/cards/cooking.webp"));
        assert!(!is_portable_asset_path("assets/../secret.png"));
        assert!(!is_portable_asset_path("assets/CON.png"));
        assert!(!is_portable_asset_path("assets/con.png"));
        assert!(!is_portable_asset_path("assets/My Card.png"));
    }
}

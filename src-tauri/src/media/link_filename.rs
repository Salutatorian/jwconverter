const MAX_STEM_LENGTH: usize = 120;

/// Creates a portable, Windows-safe filename stem from remote media metadata.
pub fn sanitize_link_stem(title: &str) -> String {
    let mut stem = title
        .chars()
        .flat_map(|character| {
            if matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            ) {
                " - ".chars().collect::<Vec<_>>()
            } else if character.is_control() {
                " ".chars().collect::<Vec<_>>()
            } else {
                vec![character]
            }
        })
        .collect::<String>();

    stem = stem.trim_end_matches(['.', ' ']).trim().to_string();
    if stem.is_empty() {
        return "download".to_string();
    }

    let base_name = stem.split('.').next().unwrap_or_default();
    if is_windows_device_name(base_name) {
        stem.insert(0, '_');
    }

    stem.chars().take(MAX_STEM_LENGTH).collect()
}

fn is_windows_device_name(stem: &str) -> bool {
    let upper = stem.trim().to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && matches!(upper.as_bytes()[3], b'1'..=b'9'))
}

#[cfg(test)]
mod tests {
    use super::sanitize_link_stem;

    #[test]
    fn replaces_windows_invalid_characters_and_trailing_dots() {
        assert_eq!(
            sanitize_link_stem(r#"My: "video"?* <test>.  "#),
            "My -   - video -  -  -   - test -"
        );
    }

    #[test]
    fn prefixes_reserved_device_names() {
        assert_eq!(sanitize_link_stem("CON"), "_CON");
        assert_eq!(sanitize_link_stem("com1.txt"), "_com1.txt");
    }

    #[test]
    fn bounds_empty_and_overlong_stems() {
        assert_eq!(sanitize_link_stem("...   "), "download");
        assert_eq!(sanitize_link_stem(&"a".repeat(150)).chars().count(), 120);
    }
}

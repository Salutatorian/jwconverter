const MAX_STEM_LENGTH: usize = 120;

/// Creates a portable, Windows-safe filename stem from remote media metadata.
pub fn sanitize_link_stem(title: &str) -> String {
    let mut raw = String::with_capacity(title.len().min(MAX_STEM_LENGTH * 2));
    for character in title.chars() {
        if matches!(
            character,
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
        ) {
            raw.push('-');
        } else if character.is_control() || character == '\u{FEFF}' || character.is_whitespace() {
            raw.push(' ');
        } else {
            raw.push(character);
        }
        if raw.chars().count() >= MAX_STEM_LENGTH + 32 {
            break;
        }
    }

    let mut stem = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    while stem.contains(" -") || stem.contains("- ") || stem.contains("--") {
        stem = stem
            .replace(" -", "-")
            .replace("- ", "-")
            .replace("--", "-");
    }
    stem = stem
        .trim_matches(|c: char| c == '-' || c == '.' || c == ' ')
        .to_string();

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
            "My-video-test"
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

    #[test]
    fn preserves_unicode_and_collapses_whitespace() {
        assert_eq!(sanitize_link_stem("日本語タイトル"), "日本語タイトル");
        assert_eq!(sanitize_link_stem("café / résumé"), "café-résumé");
        assert_eq!(sanitize_link_stem("a   \t  b"), "a b");
    }
}

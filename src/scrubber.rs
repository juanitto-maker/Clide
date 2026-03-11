// ============================================
// scrubber.rs - Output secrets scrubber
// ============================================
// Scans any string about to leave the process (Telegram, logs, Gemini prompt)
// and replaces every known secret value with "***".
//
// Rules:
//  - Only values longer than 4 chars are redacted (avoids false positives on
//    common strings like "true", "info", "prod" that might appear as values).
//  - Matching is case-sensitive (secrets usually are).
//  - Multiple passes until no further replacements happen (handles nested).
//  - The scrubber is applied to ALL outbound text; the Gemini prompt builder
//    already substitutes ${KEY} placeholders with actual values before calling
//    the shell, so the scrubber is the last safety net.

use std::collections::HashMap;

const MIN_SECRET_LEN: usize = 5;
const REDACTED: &str = "***";

/// Replace every secret value found in `text` with `***`.
/// `secrets` is the `Config::secrets` map (key → value).
pub fn scrub(text: &str, secrets: &HashMap<String, String>) -> String {
    // Collect candidate values (long enough to be worth masking).
    let mut needles: Vec<&str> = secrets
        .values()
        .map(|v| v.as_str())
        .filter(|v| v.len() >= MIN_SECRET_LEN)
        .collect();

    // Sort longest-first so a longer token shadows its own prefix.
    needles.sort_by(|a, b| b.len().cmp(&a.len()));
    needles.dedup();

    if needles.is_empty() {
        return text.to_string();
    }

    let mut output = text.to_string();
    // Iterate until stable (handles tokens that contain other tokens).
    loop {
        let mut changed = false;
        for needle in &needles {
            if output.contains(needle) {
                output = output.replace(needle, REDACTED);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_basic_redaction() {
        let secrets = map(&[("TOKEN", "super_secret_token_abc")]);
        let result = scrub("The token is super_secret_token_abc, use it!", &secrets);
        assert_eq!(result, "The token is ***, use it!");
    }

    #[test]
    fn test_short_value_not_redacted() {
        let secrets = map(&[("FLAG", "on")]);
        let result = scrub("Feature is on", &secrets);
        assert_eq!(result, "Feature is on"); // "on" is too short
    }

    #[test]
    fn test_multiple_secrets() {
        let secrets = map(&[
            ("KEY_A", "secret_key_aaaaa"),
            ("KEY_B", "another_secret_bbb"),
        ]);
        let result = scrub("a=secret_key_aaaaa b=another_secret_bbb", &secrets);
        assert_eq!(result, "a=*** b=***");
    }

    #[test]
    fn test_no_false_positive() {
        let secrets = map(&[("GEMINI_API_KEY", "AIzaSyXXXXXXXXXX")]);
        let result = scrub("Platform: telegram", &secrets);
        assert_eq!(result, "Platform: telegram");
    }

    #[test]
    fn test_empty_secrets() {
        let secrets = HashMap::new();
        let text = "nothing to scrub";
        assert_eq!(scrub(text, &secrets), text);
    }
}

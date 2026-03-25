// ============================================
// output_utils.rs - Smart Output Preprocessing
// ============================================
// Cleans and condenses command output before feeding it back to Gemini,
// preventing the model from being overwhelmed by verbose, noisy output.

/// Strip ANSI escape sequences (colors, cursor movement, etc.) from text.
pub fn strip_ansi(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // ESC[ ... <letter> — CSI sequence (most common: colors, cursor)
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Consume until we hit a letter (the terminator)
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c.is_ascii_alphabetic() || c == '~' {
                        break;
                    }
                }
            } else {
                // Other ESC sequences (e.g. ESC(B) — consume one more char
                chars.next();
            }
        } else if ch == '\r' {
            // Strip carriage returns (common in progress bars)
            continue;
        } else {
            result.push(ch);
        }
    }

    result
}

/// Collapse runs of 3+ consecutive blank lines into a single blank line.
pub fn collapse_blank_lines(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut consecutive_blanks = 0u32;

    for line in input.lines() {
        if line.trim().is_empty() {
            consecutive_blanks += 1;
            if consecutive_blanks <= 2 {
                result.push('\n');
            }
        } else {
            consecutive_blanks = 0;
            result.push_str(line);
            result.push('\n');
        }
    }

    // Remove trailing newline to match original trimmed style
    if result.ends_with('\n') {
        result.pop();
    }
    result
}

/// For large outputs (> threshold bytes), extract error/warning lines first
/// and put the rest in a condensed section. Returns the processed output.
///
/// If the output is small enough, returns it unchanged.
pub fn smart_truncate(output: &str, max_bytes: usize) -> String {
    if output.len() <= max_bytes {
        return output.to_string();
    }

    // Extract lines containing error/warning indicators
    let mut error_lines: Vec<&str> = Vec::new();
    let mut other_lines: Vec<&str> = Vec::new();

    for line in output.lines() {
        let lower = line.to_lowercase();
        if lower.contains("error")
            || lower.contains("err:")
            || lower.contains("fatal")
            || lower.contains("failed")
            || lower.contains("failure")
            || lower.contains("warning")
            || lower.contains("warn:")
            || lower.contains("panic")
            || lower.contains("traceback")
            || lower.contains("exception")
            || lower.contains("denied")
            || lower.contains("not found")
            || lower.contains("no such")
            || lower.contains("cannot ")
            || lower.contains("couldn't")
        {
            error_lines.push(line);
        } else {
            other_lines.push(line);
        }
    }

    let mut result = String::new();

    if !error_lines.is_empty() {
        result.push_str("=== ERRORS/WARNINGS ===\n");
        for line in &error_lines {
            result.push_str(line);
            result.push('\n');
        }
        result.push('\n');
    }

    // Fill remaining budget with other lines (head + tail)
    let error_size = result.len();
    let remaining_budget = max_bytes.saturating_sub(error_size + 100); // 100 for labels

    if !other_lines.is_empty() {
        result.push_str("=== OUTPUT (condensed) ===\n");

        let total_other: usize = other_lines.iter().map(|l| l.len() + 1).sum();
        if total_other <= remaining_budget {
            for line in &other_lines {
                result.push_str(line);
                result.push('\n');
            }
        } else {
            // Take head and tail portions
            let half = remaining_budget / 2;
            let mut head = String::new();
            for line in &other_lines {
                if head.len() + line.len() + 1 > half {
                    break;
                }
                head.push_str(line);
                head.push('\n');
            }

            let mut tail_lines: Vec<&str> = Vec::new();
            let mut tail_size = 0usize;
            for line in other_lines.iter().rev() {
                if tail_size + line.len() + 1 > half {
                    break;
                }
                tail_lines.push(line);
                tail_size += line.len() + 1;
            }
            tail_lines.reverse();

            result.push_str(&head);
            let skipped = other_lines.len() - head.lines().count() - tail_lines.len();
            if skipped > 0 {
                result.push_str(&format!("\n[... {} lines omitted ...]\n\n", skipped));
            }
            for line in &tail_lines {
                result.push_str(line);
                result.push('\n');
            }
        }
    }

    result
}

/// Full preprocessing pipeline: strip ANSI → collapse blanks → smart truncate.
pub fn preprocess_output(output: &str, max_bytes: usize) -> String {
    let cleaned = strip_ansi(output);
    let collapsed = collapse_blank_lines(&cleaned);
    smart_truncate(&collapsed, max_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_colors() {
        let input = "\x1b[31mError\x1b[0m: something failed";
        assert_eq!(strip_ansi(input), "Error: something failed");
    }

    #[test]
    fn test_strip_ansi_empty() {
        assert_eq!(strip_ansi("hello world"), "hello world");
    }

    #[test]
    fn test_collapse_blank_lines() {
        let input = "line1\n\n\n\n\nline2\n\nline3";
        let result = collapse_blank_lines(input);
        assert_eq!(result, "line1\n\n\nline2\n\nline3");
    }

    #[test]
    fn test_small_output_unchanged() {
        let input = "small output";
        assert_eq!(smart_truncate(input, 1000), "small output");
    }

    #[test]
    fn test_preprocess_pipeline() {
        let input = "\x1b[32mOK\x1b[0m\n\n\n\n\nDone";
        let result = preprocess_output(input, 10000);
        assert_eq!(result, "OK\n\n\nDone");
    }
}

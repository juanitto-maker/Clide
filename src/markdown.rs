// ============================================
// markdown.rs - Markdown-to-HTML converter
// Converts Gemini's Markdown output to HTML
// suitable for Telegram and Matrix messages.
// ============================================

/// Escape special HTML characters in a string.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Convert a Markdown string (as produced by Gemini) to HTML.
///
/// Supported patterns:
/// - Fenced code blocks (``` … ```) → `<pre>…</pre>`
/// - Inline code (`` `…` ``) → `<code>…</code>`
/// - Bold (`**…**`) → `<b>…</b>`
/// - Italic (`*…*`) → `<i>…</i>`
/// - ATX headers (`# `, `## `, `### `) → `<b>…</b>`
/// - All literal text is HTML-escaped.
pub fn markdown_to_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    let mut lines = text.lines().peekable();
    let mut in_code_block = false;

    while let Some(line) = lines.next() {
        if in_code_block {
            // Detect closing fence (optionally with language label on opening).
            if line.trim_start().starts_with("```") {
                // Remove the trailing newline we added for the last content line.
                if out.ends_with('\n') {
                    out.pop();
                }
                out.push_str("</pre>");
                in_code_block = false;
            } else {
                out.push_str(&html_escape(line));
                out.push('\n');
            }
        } else if line.trim_start().starts_with("```") {
            // Opening fence — skip optional language identifier on the same line.
            out.push_str("<pre>");
            in_code_block = true;
        } else if let Some(rest) = line.strip_prefix("### ") {
            out.push_str("<b>");
            out.push_str(&process_inline(rest));
            out.push_str("</b>\n");
        } else if let Some(rest) = line.strip_prefix("## ") {
            out.push_str("<b>");
            out.push_str(&process_inline(rest));
            out.push_str("</b>\n");
        } else if let Some(rest) = line.strip_prefix("# ") {
            out.push_str("<b>");
            out.push_str(&process_inline(rest));
            out.push_str("</b>\n");
        } else {
            out.push_str(&process_inline(line));
            out.push('\n');
        }
    }

    // Close an unclosed fenced code block.
    if in_code_block {
        if out.ends_with('\n') {
            out.pop();
        }
        out.push_str("</pre>");
    }

    // Remove a trailing newline added by the last line.
    if out.ends_with('\n') {
        out.pop();
    }

    out
}

/// Process inline Markdown tokens on a single line.
///
/// Handles: bold (`**…**`), inline code (`` `…` ``), italic (`*…*`).
/// All non-token text is HTML-escaped character by character.
fn process_inline(text: &str) -> String {
    let mut out = String::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Bold: **text**
        if remaining.starts_with("**") {
            let inner = &remaining[2..];
            if let Some(end) = inner.find("**") {
                out.push_str("<b>");
                out.push_str(&html_escape(&inner[..end]));
                out.push_str("</b>");
                remaining = &inner[end + 2..];
                continue;
            }
        }

        // Inline code: `text`  (but not opening ```)
        if remaining.starts_with('`') && !remaining.starts_with("```") {
            let inner = &remaining[1..];
            if let Some(end) = inner.find('`') {
                out.push_str("<code>");
                out.push_str(&html_escape(&inner[..end]));
                out.push_str("</code>");
                remaining = &inner[end + 1..];
                continue;
            }
        }

        // Italic: *text*  (must not be the start of **)
        if remaining.starts_with('*') && !remaining.starts_with("**") {
            let inner = &remaining[1..];
            // Avoid matching across a ** sequence.
            if let Some(end) = inner.find('*') {
                if !inner[end..].starts_with("**") {
                    out.push_str("<i>");
                    out.push_str(&html_escape(&inner[..end]));
                    out.push_str("</i>");
                    remaining = &inner[end + 1..];
                    continue;
                }
            }
        }

        // Emit next character, HTML-escaped.
        let c = remaining.chars().next().unwrap();
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
        remaining = &remaining[c.len_utf8()..];
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        assert_eq!(markdown_to_html("hello world"), "hello world");
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(markdown_to_html("a < b & c > d"), "a &lt; b &amp; c &gt; d");
    }

    #[test]
    fn test_bold() {
        assert_eq!(markdown_to_html("**bold**"), "<b>bold</b>");
    }

    #[test]
    fn test_italic() {
        assert_eq!(markdown_to_html("*italic*"), "<i>italic</i>");
    }

    #[test]
    fn test_inline_code() {
        assert_eq!(markdown_to_html("`code`"), "<code>code</code>");
    }

    #[test]
    fn test_header() {
        assert_eq!(markdown_to_html("## Title"), "<b>Title</b>");
    }

    #[test]
    fn test_code_block() {
        let input = "```bash\nls -la\n```";
        let output = markdown_to_html(input);
        assert!(output.contains("<pre>"));
        assert!(output.contains("ls -la"));
        assert!(output.contains("</pre>"));
        assert!(!output.contains("```"));
    }

    #[test]
    fn test_mixed() {
        let input = "Result: **success**\n```\noutput\n```";
        let output = markdown_to_html(input);
        assert!(output.contains("<b>success</b>"));
        assert!(output.contains("<pre>"));
        assert!(output.contains("output"));
    }
}

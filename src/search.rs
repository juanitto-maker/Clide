// ============================================
// search.rs - DuckDuckGo Web Search
// ============================================
// Provides a lightweight web search tool using DuckDuckGo's HTML endpoint.
// No API key required. Returns titles, URLs, and snippets.

use anyhow::Result;
use log::warn;
use reqwest::Client;

/// A single search result with title, URL, and snippet.
#[derive(Debug)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Maximum number of results to return per query.
const MAX_RESULTS: usize = 8;

/// Search DuckDuckGo and return parsed results.
///
/// Uses the HTML endpoint at `html.duckduckgo.com/html/` which requires no
/// API key and returns organic web results. Results are parsed with basic
/// string extraction (no heavy HTML parser dependency).
pub async fn search(client: &Client, query: &str) -> Result<Vec<SearchResult>> {
    let resp = client
        .post("https://html.duckduckgo.com/html/")
        .header("User-Agent", "Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36")
        .form(&[("q", query)])
        .send()
        .await?
        .text()
        .await?;

    Ok(parse_results(&resp))
}

/// Format search results into a readable string for the agent.
pub fn format_results(results: &[SearchResult]) -> String {
    if results.is_empty() {
        return "No results found.".to_string();
    }

    results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "{}. {}\n   {}\n   {}",
                i + 1,
                r.title,
                r.url,
                r.snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Parse DuckDuckGo HTML response into structured results.
///
/// The HTML contains result blocks like:
/// ```html
/// <a rel="nofollow" class="result__a" href="...">Title</a>
/// <a class="result__snippet" href="...">Snippet text</a>
/// ```
fn parse_results(html: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // Split on result link markers
    let parts: Vec<&str> = html.split("class=\"result__a\"").collect();

    // Skip the first part (before any results)
    for part in parts.iter().skip(1).take(MAX_RESULTS) {
        let title = extract_tag_text(part);
        let url = extract_href(part);
        let snippet = extract_snippet(part);

        if !title.is_empty() || !url.is_empty() {
            results.push(SearchResult {
                title: decode_html_entities(&title),
                url: clean_url(&url),
                snippet: decode_html_entities(&snippet),
            });
        }
    }

    if results.is_empty() {
        warn!("DuckDuckGo search: no results parsed from HTML response");
    }

    results
}

/// Extract the text content between > and </a> from the start of a string fragment.
fn extract_tag_text(s: &str) -> String {
    // Pattern: href="...">TEXT</a>
    if let Some(start) = s.find('>') {
        let after = &s[start + 1..];
        if let Some(end) = after.find("</a>") {
            let text = &after[..end];
            // Strip any inner HTML tags
            return strip_tags(text).trim().to_string();
        }
    }
    String::new()
}

/// Extract href="..." value from a fragment.
fn extract_href(s: &str) -> String {
    if let Some(start) = s.find("href=\"") {
        let after = &s[start + 6..];
        if let Some(end) = after.find('"') {
            return after[..end].to_string();
        }
    }
    String::new()
}

/// Extract the snippet text from a result block.
fn extract_snippet(s: &str) -> String {
    if let Some(pos) = s.find("class=\"result__snippet\"") {
        let after = &s[pos..];
        return extract_tag_text(after);
    }
    String::new()
}

/// Clean a DuckDuckGo redirect URL to extract the actual destination.
/// DDG wraps URLs like: //duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com&...
fn clean_url(url: &str) -> String {
    if let Some(pos) = url.find("uddg=") {
        let encoded = &url[pos + 5..];
        let encoded = if let Some(end) = encoded.find('&') {
            &encoded[..end]
        } else {
            encoded
        };
        return url_decode(encoded);
    }
    url.to_string()
}

/// Basic percent-decoding for URLs.
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

/// Strip HTML tags from a string.
fn strip_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
}

/// Decode common HTML entities.
fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}

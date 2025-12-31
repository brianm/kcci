use plist::Value;
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

use crate::db::ImportedBook;
use crate::error::{OokError, Result};

// Static regexes for HTML parsing (compiled once)
static SCRIPT_RE: OnceLock<Regex> = OnceLock::new();
static TITLE_RE: OnceLock<Regex> = OnceLock::new();
static AUTHOR_RE: OnceLock<Regex> = OnceLock::new();
static ASIN_RE: OnceLock<Regex> = OnceLock::new();

fn get_script_regex() -> &'static Regex {
    SCRIPT_RE.get_or_init(|| {
        // (?s) enables DOTALL mode so . matches newlines
        Regex::new(r#"(?s)<script[^>]*id="itemViewResponse"[^>]*>(.*?)</script>"#)
            .expect("Invalid script regex")
    })
}

fn get_title_regex() -> &'static Regex {
    TITLE_RE.get_or_init(|| {
        Regex::new(r#"id="title-(B0[A-Z0-9]{8,9})"><p[^>]*>([^<]+)</p>"#)
            .expect("Invalid title regex")
    })
}

fn get_author_regex() -> &'static Regex {
    AUTHOR_RE.get_or_init(|| {
        Regex::new(r#"id="author-(B0[A-Z0-9]{8,9})"><p[^>]*>([^<]+)</p>"#)
            .expect("Invalid author regex")
    })
}

fn get_asin_regex() -> &'static Regex {
    ASIN_RE.get_or_init(|| Regex::new(r#"id="title-(B0[A-Z0-9]{8,9})""#).expect("Invalid asin regex"))
}

/// Detected file format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportFormat {
    /// Safari webarchive (binary plist)
    WebArchive,
    /// MHTML (Chrome/Firefox "Save as Single File")
    Mhtml,
    /// Plain HTML
    Html,
}

/// Detect file format from content
fn detect_format(data: &[u8]) -> ImportFormat {
    // Check for binary plist magic (bplist)
    if data.starts_with(b"bplist") {
        return ImportFormat::WebArchive;
    }

    // Check for MHTML header
    let header = String::from_utf8_lossy(&data[..data.len().min(500)]);
    if header.contains("MIME-Version:") || header.contains("multipart/related") {
        return ImportFormat::Mhtml;
    }

    // Default to HTML
    ImportFormat::Html
}

/// Parse any supported import file and extract Kindle library books
pub fn parse_import_file(path: &Path) -> Result<Vec<ImportedBook>> {
    let data = std::fs::read(path)?;
    let format = detect_format(&data);

    log::info!("Detected import format: {:?} for {:?}", format, path);

    match format {
        ImportFormat::WebArchive => parse_webarchive_data(&data),
        ImportFormat::Mhtml => parse_mhtml_data(&data),
        ImportFormat::Html => {
            let html = String::from_utf8_lossy(&data);
            extract_books_from_html(&html)
        }
    }
}

/// Parse Safari webarchive from raw bytes
fn parse_webarchive_data(data: &[u8]) -> Result<Vec<ImportedBook>> {
    let plist = plist::from_bytes::<Value>(data)
        .map_err(|e| OokError::Webarchive(format!("Failed to parse plist: {}", e)))?;

    let html_bytes = plist
        .as_dictionary()
        .and_then(|d| d.get("WebMainResource"))
        .and_then(|r| r.as_dictionary())
        .and_then(|d| d.get("WebResourceData"))
        .and_then(|d| d.as_data())
        .ok_or_else(|| OokError::Webarchive("Missing WebResourceData in webarchive".into()))?;

    let html = String::from_utf8_lossy(html_bytes);
    extract_books_from_html(&html)
}

/// Parse MHTML file from raw bytes
fn parse_mhtml_data(data: &[u8]) -> Result<Vec<ImportedBook>> {
    let content = String::from_utf8_lossy(data);

    // Decode quoted-printable encoding used in MHTML
    let decoded = decode_quoted_printable(&content);

    extract_books_from_dom(&decoded)
}

/// Decode quoted-printable encoding
fn decode_quoted_printable(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '=' {
            // Check for soft line break (= at end of line)
            if chars.peek() == Some(&'\r') || chars.peek() == Some(&'\n') {
                // Skip the line break
                if chars.peek() == Some(&'\r') {
                    chars.next();
                }
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                continue;
            }

            // Try to decode hex pair
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            // If decoding failed, keep original
            result.push('=');
            result.push_str(&hex);
        } else {
            result.push(c);
        }
    }

    result
}

/// Extract book data from Kindle library HTML (JSON strategy)
fn extract_books_from_html(html: &str) -> Result<Vec<ImportedBook>> {
    let mut books = Vec::new();
    let mut seen_asins = HashSet::new();

    // Strategy 1: Look for itemViewResponse JSON embedded in the page
    if let Some(cap) = get_script_regex().captures(html) {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&cap[1]) {
            if let Some(items) = data.get("itemsList").and_then(|v| v.as_array()) {
                for item in items {
                    if let Some(asin) = item.get("asin").and_then(|v| v.as_str()) {
                        if seen_asins.insert(asin.to_string()) {
                            books.push(ImportedBook {
                                asin: asin.to_string(),
                                title: item
                                    .get("title")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                authors: extract_authors_from_json(item),
                                cover_url: None, // Don't extract cover URLs
                                percentage_read: item
                                    .get("percentageRead")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0)
                                    as i32,
                                resource_type: item
                                    .get("resourceType")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("EBOOK")
                                    .to_string(),
                                origin_type: item
                                    .get("originType")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("PURCHASE")
                                    .to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    // If JSON parsing found books, return them
    if !books.is_empty() {
        return Ok(books);
    }

    // Strategy 2: Fall back to DOM extraction
    extract_books_from_dom(html)
}

/// Extract book data from rendered DOM (for MHTML or lazy-loaded content)
fn extract_books_from_dom(html: &str) -> Result<Vec<ImportedBook>> {
    let mut books = Vec::new();
    let mut seen_asins = HashSet::new();

    // First pass: collect all ASINs from title elements
    let asins: Vec<String> = get_asin_regex()
        .captures_iter(html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect();

    // Build lookup maps for titles and authors
    let mut titles: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut authors: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for cap in get_title_regex().captures_iter(html) {
        if let (Some(asin), Some(title)) = (cap.get(1), cap.get(2)) {
            titles.insert(
                asin.as_str().to_string(),
                html_decode(title.as_str().trim()),
            );
        }
    }

    for cap in get_author_regex().captures_iter(html) {
        if let (Some(asin), Some(author)) = (cap.get(1), cap.get(2)) {
            authors.insert(
                asin.as_str().to_string(),
                html_decode(author.as_str().trim()),
            );
        }
    }

    // Build book list
    for asin in asins {
        if seen_asins.contains(&asin) {
            continue;
        }
        seen_asins.insert(asin.clone());

        let title = titles.get(&asin).cloned().unwrap_or_default();
        let author_str = authors.get(&asin).cloned().unwrap_or_default();

        // Parse authors (may be comma or colon separated)
        let author_list: Vec<String> = author_str
            .split(':')
            .flat_map(|s| s.split(','))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        books.push(ImportedBook {
            asin,
            title,
            authors: author_list,
            cover_url: None, // Don't extract cover URLs
            percentage_read: 0,
            resource_type: "EBOOK".to_string(),
            origin_type: "PURCHASE".to_string(),
        });
    }

    Ok(books)
}

/// Decode basic HTML entities
fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

/// Extract authors array from JSON item
fn extract_authors_from_json(item: &serde_json::Value) -> Vec<String> {
    item.get("authors")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim_end_matches(':').trim().to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format_webarchive() {
        let data = b"bplist00...";
        assert_eq!(detect_format(data), ImportFormat::WebArchive);
    }

    #[test]
    fn test_detect_format_mhtml() {
        let data = b"From: <Saved by Blink>\nMIME-Version: 1.0\nContent-Type: multipart/related;";
        assert_eq!(detect_format(data), ImportFormat::Mhtml);
    }

    #[test]
    fn test_detect_format_html() {
        let data = b"<!DOCTYPE html><html>...";
        assert_eq!(detect_format(data), ImportFormat::Html);
    }

    #[test]
    fn test_decode_quoted_printable() {
        assert_eq!(decode_quoted_printable("hello=20world"), "hello world");
        assert_eq!(decode_quoted_printable("line1=\r\nline2"), "line1line2");
        assert_eq!(decode_quoted_printable("foo=3Dbar"), "foo=bar");
    }

    #[test]
    fn test_extract_authors_from_json() {
        let item: serde_json::Value =
            serde_json::from_str(r#"{"authors": ["John Doe:", "Jane Smith"]}"#).unwrap();
        let authors = extract_authors_from_json(&item);
        assert_eq!(authors, vec!["John Doe", "Jane Smith"]);
    }

    #[test]
    fn test_extract_books_from_json_script() {
        let html = r#"
            <html>
            <script id="itemViewResponse">
            {"itemsList": [{"asin": "B001", "title": "Test Book", "authors": ["Author One"], "percentageRead": 50}]}
            </script>
            </html>
        "#;

        let books = extract_books_from_html(html).unwrap();
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].asin, "B001");
        assert_eq!(books[0].title, "Test Book");
        assert_eq!(books[0].percentage_read, 50);
        assert!(books[0].cover_url.is_none());
    }

    #[test]
    fn test_extract_books_from_dom() {
        let html = r#"
            <div id="title-B0TESTBOOK1"><p class="title">My Test Book</p></div>
            <div id="author-B0TESTBOOK1"><p class="author">Test Author</p></div>
        "#;

        let books = extract_books_from_dom(html).unwrap();
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].asin, "B0TESTBOOK1");
        assert_eq!(books[0].title, "My Test Book");
        assert_eq!(books[0].authors, vec!["Test Author"]);
        assert!(books[0].cover_url.is_none());
    }

    #[test]
    fn test_html_decode() {
        assert_eq!(html_decode("Tom &amp; Jerry"), "Tom & Jerry");
        assert_eq!(html_decode("It&#39;s great"), "It's great");
    }
}

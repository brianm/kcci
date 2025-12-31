use plist::Value;
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

use crate::db::ImportedBook;
use crate::error::{KcciError, Result};

// Static regexes for HTML parsing (compiled once)
static SCRIPT_RE: OnceLock<Regex> = OnceLock::new();
static COVER_RE: OnceLock<Regex> = OnceLock::new();

fn get_script_regex() -> &'static Regex {
    SCRIPT_RE.get_or_init(|| {
        // (?s) enables DOTALL mode so . matches newlines
        Regex::new(r#"(?s)<script[^>]*id="itemViewResponse"[^>]*>(.*?)</script>"#)
            .expect("Invalid script regex")
    })
}

fn get_cover_regex() -> &'static Regex {
    COVER_RE.get_or_init(|| {
        Regex::new(r#"id="coverContainer-([A-Z0-9]+)""#).expect("Invalid cover regex")
    })
}

/// Parse a Safari webarchive file and extract Kindle library books
pub fn parse_webarchive(path: &Path) -> Result<Vec<ImportedBook>> {
    let html = extract_html_from_webarchive(path)?;
    let html_str = String::from_utf8_lossy(&html);
    extract_books_from_html(&html_str)
}

/// Extract the main HTML content from a Safari webarchive file
fn extract_html_from_webarchive(path: &Path) -> Result<Vec<u8>> {
    let data = std::fs::read(path)?;
    let plist = plist::from_bytes::<Value>(&data)
        .map_err(|e| KcciError::Webarchive(format!("Failed to parse plist: {}", e)))?;

    let html_bytes = plist
        .as_dictionary()
        .and_then(|d| d.get("WebMainResource"))
        .and_then(|r| r.as_dictionary())
        .and_then(|d| d.get("WebResourceData"))
        .and_then(|d| d.as_data())
        .ok_or_else(|| KcciError::Webarchive("Missing WebResourceData in webarchive".into()))?;

    Ok(html_bytes.to_vec())
}

/// Extract book data from Kindle library HTML
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
                                cover_url: item
                                    .get("productUrl")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
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

    // Strategy 2: Extract from DOM elements (for lazy-loaded content)
    for cap in get_cover_regex().captures_iter(html) {
        let asin = cap[1].to_string();
        if seen_asins.contains(&asin) {
            continue;
        }
        seen_asins.insert(asin.clone());

        let title = extract_title_for_asin(html, &asin);
        let authors = extract_authors_for_asin(html, &asin);
        let cover_url = extract_cover_for_asin(html, &asin);

        books.push(ImportedBook {
            asin,
            title,
            authors,
            cover_url,
            percentage_read: 0,
            resource_type: "EBOOK".to_string(),
            origin_type: "PURCHASE".to_string(),
        });
    }

    Ok(books)
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

/// Extract title for a specific ASIN from DOM
fn extract_title_for_asin(html: &str, asin: &str) -> String {
    let pattern = format!(r#"id="title-{}"[^>]*>.*?<p[^>]*>([^<]+)</p>"#, asin);
    if let Ok(re) = Regex::new(&pattern) {
        if let Some(cap) = re.captures(html) {
            return cap[1].trim().to_string();
        }
    }
    String::new()
}

/// Extract authors for a specific ASIN from DOM
fn extract_authors_for_asin(html: &str, asin: &str) -> Vec<String> {
    let pattern = format!(r#"id="author-{}"[^>]*>.*?<p[^>]*>([^<]+)</p>"#, asin);
    if let Ok(re) = Regex::new(&pattern) {
        if let Some(cap) = re.captures(html) {
            let author_str = cap[1].trim().trim_end_matches(':');
            return author_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }
    }
    Vec::new()
}

/// Extract cover URL for a specific ASIN from DOM
fn extract_cover_for_asin(html: &str, asin: &str) -> Option<String> {
    let pattern = format!(r#"id="cover-{}"[^>]*src="([^"]+)""#, asin);
    if let Ok(re) = Regex::new(&pattern) {
        if let Some(cap) = re.captures(html) {
            return Some(cap[1].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}

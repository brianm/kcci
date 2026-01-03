//! Parser for Amazon "Download Your Data" Kindle export
//!
//! Amazon provides Kindle library data via their "Download Your Data" feature.
//! The export contains a folder with:
//! - `Digital.Content.Ownership/*.json` - One JSON file per book with ownership info
//! - `Kindle.UnifiedLibraryIndex/datasets/*/CustomerAuthorNameRelationship.csv` - Author data

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::db::ImportedBook;
use crate::error::{OokError, Result};

/// Check if a path is an Amazon Kindle data export folder
pub fn is_amazon_export(path: &Path) -> bool {
    path.is_dir() && path.join("Digital.Content.Ownership").is_dir()
}

/// Parse Amazon Kindle data export folder and extract books
pub fn parse_amazon_export(folder_path: &Path) -> Result<Vec<ImportedBook>> {
    let ownership_dir = folder_path.join("Digital.Content.Ownership");

    if !ownership_dir.is_dir() {
        return Err(OokError::AmazonImport(
            "Missing Digital.Content.Ownership folder".into(),
        ));
    }

    // Load author data from CSV
    let authors_map = load_author_map(folder_path)?;
    log::info!("Loaded {} author entries", authors_map.len());

    // Parse all ownership JSON files
    let mut books = Vec::new();
    let mut seen_asins = std::collections::HashSet::new();

    for entry in fs::read_dir(&ownership_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Some(book) = parse_ownership_json(&path, &authors_map)? {
                // Deduplicate by ASIN (there can be multiple rights entries per book)
                if seen_asins.insert(book.asin.clone()) {
                    books.push(book);
                }
            }
        }
    }

    log::info!(
        "Parsed {} unique books from Amazon export",
        books.len()
    );

    Ok(books)
}

/// Parse a single Digital.Content.Ownership JSON file
fn parse_ownership_json(
    path: &Path,
    authors_map: &HashMap<String, Vec<String>>,
) -> Result<Option<ImportedBook>> {
    let content = fs::read_to_string(path)?;
    let data: serde_json::Value = serde_json::from_str(&content)?;

    // Extract resource info
    let resource = match data.get("resource") {
        Some(r) => r,
        None => return Ok(None),
    };

    // Only process Kindle ebooks
    let resource_type = resource
        .get("resourceType")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if resource_type != "KindleEBook" {
        return Ok(None);
    }

    // Check if any right is active (filter out revoked/returned books)
    let rights = data.get("rights").and_then(|v| v.as_array());
    let has_active_right = rights
        .map(|arr| {
            arr.iter().any(|right| {
                right
                    .get("rightStatus")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "Active")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if !has_active_right {
        return Ok(None);
    }

    // Extract ASIN
    let asin = resource
        .get("asin")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let asin = match asin {
        Some(a) => a,
        None => return Ok(None),
    };

    // Extract title (may be "Not Available")
    let title = resource
        .get("productName")
        .and_then(|v| v.as_str())
        .unwrap_or("Not Available")
        .to_string();

    // Get origin type from first active right
    let origin_type = rights
        .and_then(|arr| {
            arr.iter()
                .find(|r| {
                    r.get("rightStatus")
                        .and_then(|v| v.as_str())
                        .map(|s| s == "Active")
                        .unwrap_or(false)
                })
                .and_then(|r| r.get("origin"))
                .and_then(|o| o.get("originType"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("Purchase")
        .to_string();

    // Look up authors from the author map
    let authors = authors_map
        .get(&asin)
        .cloned()
        .unwrap_or_default();

    Ok(Some(ImportedBook {
        asin,
        title,
        authors,
        resource_type: "EBOOK".to_string(),
        origin_type,
    }))
}

/// Load author data from the CustomerAuthorNameRelationship CSV
fn load_author_map(folder_path: &Path) -> Result<HashMap<String, Vec<String>>> {
    let csv_path = folder_path
        .join("Kindle.UnifiedLibraryIndex")
        .join("datasets")
        .join("Kindle.UnifiedLibraryIndex.CustomerAuthorNameRelationship")
        .join("Kindle.UnifiedLibraryIndex.CustomerAuthorNameRelationship.csv");

    let mut authors_map: HashMap<String, Vec<String>> = HashMap::new();

    if !csv_path.exists() {
        log::warn!(
            "Author CSV not found at {:?}, books will have no author data",
            csv_path
        );
        return Ok(authors_map);
    }

    let content = fs::read_to_string(&csv_path)?;
    let mut lines = content.lines();

    // Skip header row
    if lines.next().is_none() {
        return Ok(authors_map);
    }

    for line in lines {
        if let Some((asin, author)) = parse_author_csv_line(line) {
            authors_map.entry(asin).or_default().push(author);
        }
    }

    Ok(authors_map)
}

/// Parse a single line from the author CSV
/// Format: "Product Name","ASIN","Author Name"
fn parse_author_csv_line(line: &str) -> Option<(String, String)> {
    // Simple CSV parsing - fields are quoted
    let fields: Vec<&str> = line.split(',').collect();

    if fields.len() < 3 {
        return None;
    }

    // ASIN is the second field
    let asin = fields[1].trim().trim_matches('"').to_string();

    // Author name is the third field (may contain commas, so join remaining fields)
    let author_parts: Vec<&str> = fields[2..].iter().map(|s| *s).collect();
    let author = author_parts
        .join(",")
        .trim()
        .trim_matches('"')
        .to_string();

    if asin.is_empty() || author.is_empty() {
        return None;
    }

    Some((asin, author))
}

/// Parse ownership JSON from string content (for testing)
#[cfg(test)]
fn parse_ownership_json_str(
    content: &str,
    authors_map: &HashMap<String, Vec<String>>,
) -> Result<Option<ImportedBook>> {
    let data: serde_json::Value = serde_json::from_str(content)?;

    let resource = match data.get("resource") {
        Some(r) => r,
        None => return Ok(None),
    };

    let resource_type = resource
        .get("resourceType")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if resource_type != "KindleEBook" {
        return Ok(None);
    }

    let rights = data.get("rights").and_then(|v| v.as_array());
    let has_active_right = rights
        .map(|arr| {
            arr.iter().any(|right| {
                right
                    .get("rightStatus")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "Active")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if !has_active_right {
        return Ok(None);
    }

    let asin = resource
        .get("asin")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let asin = match asin {
        Some(a) => a,
        None => return Ok(None),
    };

    let title = resource
        .get("productName")
        .and_then(|v| v.as_str())
        .unwrap_or("Not Available")
        .to_string();

    let origin_type = rights
        .and_then(|arr| {
            arr.iter()
                .find(|r| {
                    r.get("rightStatus")
                        .and_then(|v| v.as_str())
                        .map(|s| s == "Active")
                        .unwrap_or(false)
                })
                .and_then(|r| r.get("origin"))
                .and_then(|o| o.get("originType"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("Purchase")
        .to_string();

    let authors = authors_map.get(&asin).cloned().unwrap_or_default();

    Ok(Some(ImportedBook {
        asin,
        title,
        authors,
        resource_type: "EBOOK".to_string(),
        origin_type,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_author_csv_line() {
        let line = r#""Test Book","B001234567","John Doe""#;
        let result = parse_author_csv_line(line);
        assert_eq!(
            result,
            Some(("B001234567".to_string(), "John Doe".to_string()))
        );
    }

    #[test]
    fn test_parse_author_csv_line_with_comma_in_name() {
        let line = r#""Test Book","B001234567","Doe, John""#;
        let result = parse_author_csv_line(line);
        assert_eq!(
            result,
            Some(("B001234567".to_string(), "Doe, John".to_string()))
        );
    }

    #[test]
    fn test_parse_author_csv_line_empty() {
        let line = r#""","","""#;
        let result = parse_author_csv_line(line);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_author_csv_line_insufficient_fields() {
        let line = r#""Only One Field""#;
        let result = parse_author_csv_line(line);
        assert_eq!(result, None);
    }

    #[test]
    fn test_is_amazon_export_false_for_file() {
        let path = Path::new("/some/file.txt");
        assert!(!is_amazon_export(path));
    }

    #[test]
    fn test_parse_ownership_json_active_ebook() {
        let json = r#"{
            "rights": [{"rightType": "Download", "origin": {"originType": "Purchase"}, "rightStatus": "Active"}],
            "resource": {"resourceType": "KindleEBook", "asin": "B001TEST", "productName": "Test Book"}
        }"#;

        let authors = HashMap::new();
        let result = parse_ownership_json_str(json, &authors).unwrap();

        assert!(result.is_some());
        let book = result.unwrap();
        assert_eq!(book.asin, "B001TEST");
        assert_eq!(book.title, "Test Book");
        assert_eq!(book.origin_type, "Purchase");
    }

    #[test]
    fn test_parse_ownership_json_with_authors() {
        let json = r#"{
            "rights": [{"rightType": "Download", "origin": {"originType": "Purchase"}, "rightStatus": "Active"}],
            "resource": {"resourceType": "KindleEBook", "asin": "B001TEST", "productName": "Test Book"}
        }"#;

        let mut authors = HashMap::new();
        authors.insert(
            "B001TEST".to_string(),
            vec!["Author One".to_string(), "Author Two".to_string()],
        );

        let result = parse_ownership_json_str(json, &authors).unwrap();
        let book = result.unwrap();
        assert_eq!(book.authors, vec!["Author One", "Author Two"]);
    }

    #[test]
    fn test_parse_ownership_json_revoked_returns_none() {
        let json = r#"{
            "rights": [{"rightType": "Download", "origin": {"originType": "Purchase"}, "rightStatus": "Revoked"}],
            "resource": {"resourceType": "KindleEBook", "asin": "B001TEST", "productName": "Returned Book"}
        }"#;

        let authors = HashMap::new();
        let result = parse_ownership_json_str(json, &authors).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_ownership_json_non_ebook_returns_none() {
        let json = r#"{
            "rights": [{"rightType": "Download", "origin": {"originType": "Purchase"}, "rightStatus": "Active"}],
            "resource": {"resourceType": "KindleUserGuide", "asin": "B001GUIDE", "productName": "User Guide"}
        }"#;

        let authors = HashMap::new();
        let result = parse_ownership_json_str(json, &authors).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_ownership_json_missing_asin_returns_none() {
        let json = r#"{
            "rights": [{"rightType": "Download", "origin": {"originType": "Purchase"}, "rightStatus": "Active"}],
            "resource": {"resourceType": "KindleEBook", "productName": "No ASIN Book"}
        }"#;

        let authors = HashMap::new();
        let result = parse_ownership_json_str(json, &authors).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_ownership_json_missing_title_uses_default() {
        let json = r#"{
            "rights": [{"rightType": "Download", "origin": {"originType": "Purchase"}, "rightStatus": "Active"}],
            "resource": {"resourceType": "KindleEBook", "asin": "B001TEST"}
        }"#;

        let authors = HashMap::new();
        let result = parse_ownership_json_str(json, &authors).unwrap();
        let book = result.unwrap();
        assert_eq!(book.title, "Not Available");
    }

    #[test]
    fn test_parse_ownership_json_sharing_origin() {
        let json = r#"{
            "rights": [{"rightType": "Download", "origin": {"originType": "Sharing"}, "rightStatus": "Active"}],
            "resource": {"resourceType": "KindleEBook", "asin": "B001TEST", "productName": "Shared Book"}
        }"#;

        let authors = HashMap::new();
        let result = parse_ownership_json_str(json, &authors).unwrap();
        let book = result.unwrap();
        assert_eq!(book.origin_type, "Sharing");
    }

    #[test]
    fn test_parse_ownership_json_multiple_rights_one_active() {
        let json = r#"{
            "rights": [
                {"rightType": "Download", "origin": {"originType": "KindleUnlimited"}, "rightStatus": "Revoked"},
                {"rightType": "Download", "origin": {"originType": "Purchase"}, "rightStatus": "Active"}
            ],
            "resource": {"resourceType": "KindleEBook", "asin": "B001TEST", "productName": "Re-purchased Book"}
        }"#;

        let authors = HashMap::new();
        let result = parse_ownership_json_str(json, &authors).unwrap();
        let book = result.unwrap();
        assert_eq!(book.origin_type, "Purchase");
    }
}

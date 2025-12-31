use regex::Regex;
use reqwest::blocking::Client;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use crate::db::EnrichmentData;
use crate::error::Result;

const USER_AGENT: &str = "Ook/1.0 (https://github.com/brianm/ook; brianm@skife.org)";
const DEFAULT_DELAY: Duration = Duration::from_millis(250);

// Static regexes for title normalization (compiled once)
static PARENTHETICAL_RE: OnceLock<Regex> = OnceLock::new();
static SUBTITLE_RE: OnceLock<Regex> = OnceLock::new();

fn get_parenthetical_regex() -> &'static Regex {
    PARENTHETICAL_RE.get_or_init(|| Regex::new(r"\s*\([^)]*\)").expect("Invalid parenthetical regex"))
}

fn get_subtitle_regex() -> &'static Regex {
    SUBTITLE_RE.get_or_init(|| Regex::new(r":.*$").expect("Invalid subtitle regex"))
}

/// OpenLibrary API client with rate limiting
pub struct OpenLibrary {
    client: Client,
}

impl OpenLibrary {
    /// Create a new OpenLibrary client
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self { client })
    }

    /// Search for a book and fetch its metadata
    pub fn search(&self, title: &str, authors: &[String]) -> Result<Option<EnrichmentData>> {
        let clean_title = normalize_title(title);

        // Try with author first
        if !authors.is_empty() {
            let author = normalize_author(&authors[0]);
            if let Some(result) = self.search_api(&clean_title, Some(&author))? {
                return Ok(Some(result));
            }
        }

        // Fallback to title-only search
        self.search_api(&clean_title, None)
    }

    /// Perform the actual API search
    fn search_api(
        &self,
        title: &str,
        author: Option<&str>,
    ) -> Result<Option<EnrichmentData>> {
        let mut url = format!(
            "https://openlibrary.org/search.json?title={}&limit=5&fields=key,title,author_name,subject,isbn,first_publish_year",
            urlencoding::encode(title)
        );
        if let Some(a) = author {
            url.push_str(&format!("&author={}", urlencoding::encode(a)));
        }

        let Some(response) = self.request_with_backoff(&url)? else {
            return Ok(None);
        };

        let data: serde_json::Value = response.json()?;
        let docs = data.get("docs").and_then(|d| d.as_array());

        if let Some(docs) = docs {
            if let Some(first) = docs.first() {
                let work_key = first.get("key").and_then(|k| k.as_str()).unwrap_or("");

                let subjects: Vec<String> = first
                    .get("subject")
                    .and_then(|s| s.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .take(20)
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default();

                let isbn = first
                    .get("isbn")
                    .and_then(|i| i.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let publish_year = first
                    .get("first_publish_year")
                    .and_then(|y| y.as_i64())
                    .map(|y| y as i32);

                // Get description from work details
                thread::sleep(DEFAULT_DELAY);
                let description = if !work_key.is_empty() {
                    self.get_work_description(work_key)?
                } else {
                    String::new()
                };

                return Ok(Some(EnrichmentData {
                    openlibrary_key: work_key.to_string(),
                    description,
                    subjects,
                    isbn,
                    publish_year,
                }));
            }
        }

        Ok(None)
    }

    /// Fetch work description from OpenLibrary
    fn get_work_description(&self, work_key: &str) -> Result<String> {
        let url = format!("https://openlibrary.org{}.json", work_key);
        if let Some(resp) = self.request_with_backoff(&url)? {
            let data: serde_json::Value = resp.json()?;
            if let Some(desc) = data.get("description") {
                if let Some(s) = desc.as_str() {
                    return Ok(s.to_string());
                }
                if let Some(obj) = desc.as_object() {
                    if let Some(v) = obj.get("value").and_then(|v| v.as_str()) {
                        return Ok(v.to_string());
                    }
                }
            }
        }
        Ok(String::new())
    }

    /// Make HTTP request with exponential backoff on 429 errors
    fn request_with_backoff(
        &self,
        url: &str,
    ) -> Result<Option<reqwest::blocking::Response>> {
        let mut delay = Duration::from_secs(1);
        let max_retries = 5;

        for attempt in 0..max_retries {
            match self.client.get(url).send() {
                Ok(resp) if resp.status() == 429 => {
                    // Rate limited - check Retry-After header
                    if let Some(retry) = resp.headers().get("Retry-After") {
                        if let Ok(secs) = retry.to_str().unwrap_or("1").parse::<u64>() {
                            delay = Duration::from_secs(secs);
                        }
                    }
                    thread::sleep(delay);
                    delay *= 2;
                }
                Ok(resp) if resp.status().is_success() => return Ok(Some(resp)),
                Ok(_) => return Ok(None),
                Err(_) if attempt < max_retries - 1 => {
                    thread::sleep(delay);
                    delay *= 2;
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(None)
    }
}

/// Normalize title for API search (remove series info, subtitles)
fn normalize_title(title: &str) -> String {
    let cleaned = get_parenthetical_regex().replace_all(title, "");
    get_subtitle_regex().replace_all(&cleaned, "").trim().to_string()
}

/// Normalize author name (convert "Last, First" to "First Last")
fn normalize_author(author: &str) -> String {
    if let Some((last, first)) = author.split_once(',') {
        format!("{} {}", first.trim(), last.trim())
    } else {
        author.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_title() {
        assert_eq!(normalize_title("The Book (Series #1)"), "The Book");
        assert_eq!(normalize_title("Title: A Subtitle"), "Title");
        assert_eq!(
            normalize_title("Book (Edition 2): More Stuff"),
            "Book"
        );
    }

    #[test]
    fn test_normalize_author() {
        assert_eq!(normalize_author("Doe, John"), "John Doe");
        assert_eq!(normalize_author("Jane Smith"), "Jane Smith");
    }
}

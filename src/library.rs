use anyhow::Result;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde::{Serialize, Deserialize};

/**
 * 1. Do a search for the book
 * 2. Fetch the book data to get description
 *
 */

pub async fn stuff() -> Result<()> {
    let cache_manager = CACacheManager {
        path: std::path::PathBuf::from("/tmp/kcci-cache"),
        ..CACacheManager::default()
    };

    let client = ClientBuilder::new(Client::new())
        .with(Cache(HttpCache {
            mode: CacheMode::Default,
            manager: cache_manager,
            options: HttpCacheOptions::default(),
        }))
        .build();

    client
        .get("https://developer.mozilla.org/en-US/docs/Web/HTTP/Caching")
        .send()
        .await?;
    Ok(())
}

pub struct OpenLibrary {
    client: ClientWithMiddleware,
}

impl OpenLibrary {
    pub fn new() -> Self {
        let cache_manager = CACacheManager {
            path: std::path::PathBuf::from("/tmp/kcci-cache"),
            ..CACacheManager::default()
        };

        let client = ClientBuilder::new(Client::new())
            .with(Cache(HttpCache {
                mode: CacheMode::Default,
                manager: cache_manager,
                options: HttpCacheOptions::default(),
            }))
            .build();
        Self { client }
    }

    pub async fn search(&self, authors: &Vec<String>, title: &String) -> Result<Option<BookData>> {
        // First we build the query string for the title and authors
        let mut ser = url::form_urlencoded::Serializer::new(String::new());
        ser.append_pair("title", title);
        for a in authors {
            ser.append_pair("author", a);
        }
        let query = ser.finish();

        let url = format!("http://openlibrary.org/search.json?{}", query);
        let resp = self.client.get(&url).send().await?;
        let body = resp.text().await?;
        let data: Root = serde_json::from_str(&body)?;        
        if data.num_found == 0 {
            return Ok(None);
        }
        let mut bd = BookData::default();
        bd.title = data.docs[0].title.clone();
        bd.authors = data.docs[0].author_name.clone();
        bd.description = todo!("need to get desciption now!");
        
        Ok(Some(bd))
    }
}

#[derive(Deserialize, Default, Debug)]
pub struct BookData {
    title: String,
    authors: Vec<String>,
    description: String,
}

mod tests {
    #[test]
    fn st8uff() {
        let encoded: String = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("foo", "bar & baz")
            .append_pair("saison", "Été+hiver")
            .finish();
        assert_eq!(encoded, "foo=bar+%26+baz&saison=%C3%89t%C3%A9%2Bhiver");
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Root {
    #[serde(rename = "numFound")]
    pub num_found: i64,
    pub start: i64,
    #[serde(rename = "numFoundExact")]
    pub num_found_exact: bool,
    pub docs: Vec<Doc>,
    #[serde(rename = "num_found")]
    pub num_found2: i64,
    pub q: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Doc {
    pub key: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub seed: Vec<String>,
    pub title: String,
    pub title_suggest: String,
    pub title_sort: String,
    pub edition_count: i64,
    pub edition_key: Vec<String>,
    pub publish_date: Vec<String>,
    pub publish_year: Vec<i64>,
    pub first_publish_year: i64,
    pub number_of_pages_median: Option<i64>,
    #[serde(default)]
    pub lccn: Vec<String>,
    #[serde(default)]
    pub oclc: Vec<String>,
    #[serde(default)]
    pub lcc: Vec<String>,
    #[serde(default)]
    pub ddc: Vec<String>,
    pub isbn: Vec<String>,
    pub last_modified_i: i64,
    pub ebook_count_i: i64,
    pub ebook_access: String,
    pub has_fulltext: bool,
    pub public_scan_b: bool,
    pub ratings_count_1: Option<i64>,
    pub ratings_count_2: Option<i64>,
    pub ratings_count_3: Option<i64>,
    pub ratings_count_4: Option<i64>,
    pub ratings_count_5: Option<i64>,
    pub ratings_average: Option<f64>,
    pub ratings_sortable: Option<f64>,
    pub ratings_count: Option<i64>,
    pub readinglog_count: Option<i64>,
    pub want_to_read_count: Option<i64>,
    pub currently_reading_count: Option<i64>,
    pub already_read_count: Option<i64>,
    pub cover_edition_key: Option<String>,
    pub cover_i: Option<i64>,
    pub publisher: Vec<String>,
    #[serde(default)]
    pub language: Vec<String>,
    pub author_key: Vec<String>,
    pub author_name: Vec<String>,
    pub subject: Vec<String>,
    #[serde(default)]
    pub id_amazon: Vec<String>,
    pub publisher_facet: Vec<String>,
    pub subject_facet: Vec<String>,
    #[serde(rename = "_version_")]
    pub version: i64,
    pub lcc_sort: Option<String>,
    pub author_facet: Vec<String>,
    pub subject_key: Vec<String>,
    pub ddc_sort: Option<String>,
}
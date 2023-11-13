use anyhow::Result;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde::Deserialize;

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
        let data = serde_json::from_str(&body)?;
        Ok(data)
    }
}

#[derive(Deserialize)]
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

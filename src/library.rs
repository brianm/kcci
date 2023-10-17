use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, Result};

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

use bytes::Bytes;
use rustysquid::{create_cache_key, CachedResponse, ProxyCache};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Create a new cache instance
    let cache = ProxyCache::new();

    // Create a cache key for a request
    let key = create_cache_key("example.com", 80, "/index.html");

    // Check if we have a cached response
    if let Some(cached) = cache.get(key).await {
        println!("Cache hit! Status: {}", cached.status_line);
    } else {
        println!("Cache miss, fetching from upstream...");

        // Simulate fetching from upstream
        let response = CachedResponse {
            status_line: "HTTP/1.1 200 OK\r\n".to_string(),
            headers: vec![
                "Content-Type: text/html".to_string(),
                "Cache-Control: max-age=3600".to_string(),
            ],
            body: Bytes::from("<html><body>Hello World</body></html>"),
            expires: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600,
        };

        // Store in cache
        if cache.put(key, response).await {
            println!("Response cached successfully");
        } else {
            println!("Failed to cache (too large or memory pressure)");
        }
    }

    // Check cache statistics
    println!("Cache entries: {}", cache.len().await);
    println!("Cache size: {} bytes", cache.total_size());
}

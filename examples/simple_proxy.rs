/// Simple proxy example - demonstrates basic HTTP proxying
///
/// Run with: cargo run --example simple_proxy
/// Then test with: curl -x localhost:3128 http://example.com
use rustysquid::{create_cache_key, ProxyCache};
use std::time::Instant;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("rustysquid=info")
        .init();

    println!("🚀 RustySquid Simple Proxy Example");
    println!("================================");
    println!("Starting proxy on localhost:3128");

    // Create cache
    let cache = ProxyCache::new();
    println!("✅ Cache initialized with capacity: 10000");

    // Simulate some cache operations
    let start = Instant::now();

    // Create a cache key
    let key = create_cache_key("example.com", 80, "/index.html");
    println!("📝 Generated cache key for example.com: {}", key);

    // Check if empty
    if cache.is_empty().await {
        println!("✅ Cache is initially empty");
    }

    // Simulate adding an entry
    use bytes::Bytes;
    use rustysquid::CachedResponse;
    use std::time::{SystemTime, UNIX_EPOCH};

    let response = CachedResponse {
        status_line: "HTTP/1.1 200 OK\r\n".to_string(),
        headers: vec![
            "Content-Type: text/html".to_string(),
            "Cache-Control: max-age=3600".to_string(),
        ],
        body: Bytes::from("<html><body>Hello from cache!</body></html>"),
        expires: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600,
    };

    if cache.put(key, response.clone()).await {
        println!("✅ Added response to cache");
    }

    // Retrieve from cache
    if let Some(cached) = cache.get(key).await {
        println!("✅ Retrieved from cache: {} bytes", cached.body.len());
        println!("   Status: {}", cached.status_line.trim());
        println!("   Headers: {} items", cached.headers.len());
    }

    // Show cache stats
    println!("\n📊 Cache Statistics:");
    println!("   Entries: {}", cache.len().await);
    println!("   Total size: {} bytes", cache.total_size());
    println!("   Time elapsed: {:?}", start.elapsed());

    println!("\n✨ Example completed successfully!");
    println!("\nTo run a full proxy server, use: cargo run");
}

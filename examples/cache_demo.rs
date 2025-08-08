use bytes::Bytes;
/// Cache demonstration - shows cache operations and TTL handling
///
/// Run with: cargo run --example cache_demo
use rustysquid::{
    calculate_ttl, create_cache_key, extract_host, is_cacheable, CachedResponse, ProxyCache,
    CACHE_SIZE, CACHE_TTL, MAX_CACHE_BYTES, MAX_ENTRY_SIZE,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("🔧 RustySquid Cache Demonstration");
    println!("==================================");

    // Show configuration
    println!("\n📋 Cache Configuration:");
    println!("   Max entries: {}", CACHE_SIZE);
    println!("   Max total size: {} MB", MAX_CACHE_BYTES / 1_048_576);
    println!("   Max entry size: {} MB", MAX_ENTRY_SIZE / 1_048_576);
    println!("   Default TTL: {} seconds", CACHE_TTL);

    let cache = ProxyCache::new();

    // Demonstrate cacheability checks
    println!("\n🔍 Cacheability Tests:");
    test_cacheability();

    // Demonstrate TTL calculation
    println!("\n⏱️ TTL Calculation:");
    test_ttl_calculation();

    // Demonstrate cache operations
    println!("\n💾 Cache Operations:");
    test_cache_operations(&cache).await;

    // Demonstrate expiration
    println!("\n⏰ Cache Expiration:");
    test_cache_expiration(&cache).await;

    // Demonstrate size limits
    println!("\n📏 Size Limits:");
    test_size_limits(&cache).await;

    println!("\n✅ All demonstrations completed!");
}

fn test_cacheability() {
    // Test different scenarios
    let tests = vec![
        (
            "GET",
            "/index.html",
            vec![],
            true,
            "HTML files are cacheable",
        ),
        ("GET", "/style.css", vec![], true, "CSS files are cacheable"),
        ("GET", "/image.jpg", vec![], true, "Images are cacheable"),
        (
            "POST",
            "/api/data",
            vec![],
            false,
            "POST requests not cacheable",
        ),
        (
            "GET",
            "/data",
            vec!["Cache-Control: no-cache".to_string()],
            false,
            "no-cache respected",
        ),
        (
            "GET",
            "/user",
            vec!["Cache-Control: private".to_string()],
            false,
            "private not cached",
        ),
        (
            "GET",
            "/api",
            vec!["Cache-Control: max-age=300".to_string()],
            true,
            "max-age allows caching",
        ),
    ];

    for (method, path, headers, expected, reason) in tests {
        let result = is_cacheable(method, path, &headers);
        let status = if result == expected { "✅" } else { "❌" };
        println!(
            "   {} {} {} - {} ({})",
            status, method, path, result, reason
        );
    }
}

fn test_ttl_calculation() {
    let tests = vec![
        (vec![], CACHE_TTL, "Default TTL when no headers"),
        (
            vec!["Cache-Control: max-age=300".to_string()],
            300,
            "5 minutes from max-age",
        ),
        (
            vec!["Cache-Control: max-age=7200".to_string()],
            7200,
            "2 hours from max-age",
        ),
        (
            vec!["Cache-Control: max-age=100000".to_string()],
            86400,
            "Capped at 24 hours",
        ),
    ];

    for (headers, expected, description) in tests {
        let ttl = calculate_ttl(&headers);
        let status = if ttl == expected { "✅" } else { "❌" };
        println!("   {} TTL: {}s - {}", status, ttl, description);
    }
}

async fn test_cache_operations(cache: &ProxyCache) {
    // Add multiple entries
    for i in 0..5 {
        let key = create_cache_key(&format!("site{}.com", i), 80, "/page");
        let response = CachedResponse {
            status_line: format!("HTTP/1.1 200 OK {}\r\n", i),
            headers: vec!["Content-Type: text/html".to_string()],
            body: Bytes::from(format!("Content {}", i)),
            expires: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600,
        };

        if cache.put(key, response).await {
            println!("   ✅ Added entry {} to cache", i);
        }
    }

    println!("   📊 Cache now has {} entries", cache.len().await);
    println!("   📊 Total size: {} bytes", cache.total_size());

    // Test retrieval
    let test_key = create_cache_key("site2.com", 80, "/page");
    if let Some(cached) = cache.get(test_key).await {
        println!(
            "   ✅ Successfully retrieved: {}",
            cached.status_line.trim()
        );
    }

    // Clear cache
    cache.clear().await;
    println!(
        "   🗑️ Cache cleared - now has {} entries",
        cache.len().await
    );
}

async fn test_cache_expiration(cache: &ProxyCache) {
    // Add an entry that expires in 1 second
    let key = create_cache_key("expire.com", 80, "/test");
    let response = CachedResponse {
        status_line: "HTTP/1.1 200 OK\r\n".to_string(),
        headers: vec![],
        body: Bytes::from("Will expire soon"),
        expires: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 1,
    };

    cache.put(key, response).await;
    println!("   ✅ Added entry with 1 second TTL");

    // Should be retrievable immediately
    if cache.get(key).await.is_some() {
        println!("   ✅ Entry retrievable immediately");
    }

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should not be retrievable after expiration
    if cache.get(key).await.is_none() {
        println!("   ✅ Entry correctly expired after TTL");
    }
}

async fn test_size_limits(cache: &ProxyCache) {
    // Try to add an oversized entry
    let key = create_cache_key("large.com", 80, "/huge");
    let oversized = CachedResponse {
        status_line: "HTTP/1.1 200 OK\r\n".to_string(),
        headers: vec![],
        body: Bytes::from(vec![0u8; MAX_ENTRY_SIZE + 1]),
        expires: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600,
    };

    if !cache.put(key, oversized).await {
        println!(
            "   ✅ Correctly rejected oversized entry (>{} MB)",
            MAX_ENTRY_SIZE / 1_048_576
        );
    }

    // Add a normal-sized entry
    let normal = CachedResponse {
        status_line: "HTTP/1.1 200 OK\r\n".to_string(),
        headers: vec![],
        body: Bytes::from(vec![0u8; 1024]),
        expires: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600,
    };

    if cache.put(key, normal).await {
        println!("   ✅ Accepted normal-sized entry (1 KB)");
    }
}

fn test_host_extraction() {
    println!("\n🌐 Host Extraction Tests:");

    let tests = vec![
        (
            vec!["Host: example.com".to_string()],
            Some(("example.com".to_string(), 80)),
        ),
        (
            vec!["Host: example.com:8080".to_string()],
            Some(("example.com".to_string(), 8080)),
        ),
        (vec!["Content-Type: text/html".to_string()], None),
    ];

    for (headers, expected) in tests {
        let result = extract_host(&headers);
        let status = if result == expected { "✅" } else { "❌" };
        match result {
            Some((host, port)) => println!("   {} Extracted: {}:{}", status, host, port),
            None => println!("   {} No host found", status),
        }
    }
}

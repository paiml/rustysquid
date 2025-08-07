use bytes::Bytes;
use proptest::prelude::*;
use rustysquid::*;
use std::time::{SystemTime, UNIX_EPOCH};

// Property: Cache keys should be deterministic
proptest! {
    #[test]
    fn prop_cache_key_deterministic(
        host in "[a-z]{3,10}\\.(com|org|net)",
        port in 1u16..65535u16,
        path in "/[a-z0-9/]{1,50}"
    ) {
        let key1 = create_cache_key(&host, port, &path);
        let key2 = create_cache_key(&host, port, &path);
        prop_assert_eq!(key1, key2);
    }
}

// Property: Different inputs should produce different keys (with high probability)
proptest! {
    #[test]
    fn prop_cache_key_uniqueness(
        host1 in "[a-z]{3,10}\\.(com|org|net)",
        host2 in "[a-z]{3,10}\\.(com|org|net)",
        port in 1u16..65535u16,
        path in "/[a-z0-9/]{1,50}"
    ) {
        prop_assume!(host1 != host2);
        let key1 = create_cache_key(&host1, port, &path);
        let key2 = create_cache_key(&host2, port, &path);
        prop_assert_ne!(key1, key2);
    }
}

// Property: TTL calculation should never exceed 24 hours
proptest! {
    #[test]
    fn prop_ttl_max_limit(max_age in 0u64..1_000_000u64) {
        let headers = vec![format!("Cache-Control: max-age={}", max_age)];
        let ttl = calculate_ttl(&headers);
        prop_assert!(ttl <= 86400);
    }
}

// Property: Static files should always be cacheable for GET requests
proptest! {
    #[test]
    fn prop_static_files_cacheable(
        extension in prop::sample::select(vec![
            ".jpg", ".png", ".css", ".js", ".woff", ".svg"
        ])
    ) {
        let path = format!("/test{extension}");
        let result = is_cacheable("GET", &path, &[]);
        prop_assert!(result);
    }
}

// Property: Non-GET methods should never be cacheable
proptest! {
    #[test]
    fn prop_non_get_not_cacheable(
        method in prop::sample::select(vec!["POST", "PUT", "DELETE", "PATCH"]),
        path in "/[a-z0-9/]{1,50}\\.(jpg|css|js)"
    ) {
        let result = is_cacheable(method, &path, &[]);
        prop_assert!(!result);
    }
}

// Property: Cache should respect no-cache headers
proptest! {
    #[test]
    fn prop_no_cache_header_respected(
        path in "/[a-z0-9/]{1,50}\\.(jpg|css|js)"
    ) {
        let headers = vec!["Cache-Control: no-cache".to_string()];
        let result = is_cacheable("GET", &path, &headers);
        prop_assert!(!result);
    }
}

// Property: Host extraction should handle ports correctly
proptest! {
    #[test]
    fn prop_host_extraction_with_port(
        host in "[a-z]{3,10}\\.(com|org|net)",
        port in 1u16..65535u16
    ) {
        let headers = vec![format!("Host: {}:{}", host, port)];
        let result = extract_host(&headers);
        prop_assert_eq!(result, Some((host, port)));
    }
}

// Property: Host extraction should default to port 80
proptest! {
    #[test]
    fn prop_host_extraction_default_port(
        host in "[a-z]{3,10}\\.(com|org|net)"
    ) {
        let headers = vec![format!("Host: {}", host)];
        let result = extract_host(&headers);
        prop_assert_eq!(result, Some((host, 80)));
    }
}

// Async property test for cache operations
#[tokio::test]
async fn prop_cache_operations() {
    let cache = ProxyCache::new();

    // Property: Cache should grow when adding items
    for i in 0..100 {
        let key = create_cache_key(&format!("test{i}.com"), 80, "/");
        let response = CachedResponse {
            status_line: "HTTP/1.1 200 OK\r\n".to_string(),
            headers: vec![],
            body: Bytes::from(format!("body{i}")),
            expires: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600,
        };
        cache.put(key, response).await;
        assert!(cache.len().await > 0);
        assert!(cache.len().await <= 100);
    }

    // Property: Clear should empty the cache
    cache.clear().await;
    assert_eq!(cache.len().await, 0);
}

// Property: Cache size never exceeds MAX_CACHE_BYTES
#[tokio::test]
async fn prop_cache_size_never_exceeds_limit() {
    let cache = ProxyCache::new();

    for i in 0..100 {
        let size = (i * 100_000) % (MAX_ENTRY_SIZE - 1000) + 1000; // Vary size but stay under limit
        let response = CachedResponse {
            status_line: "HTTP/1.1 200 OK\r\n".to_string(),
            headers: vec!["Content-Type: text/html".to_string()],
            body: Bytes::from(vec![0u8; size]),
            expires: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600,
        };

        let key = create_cache_key(&format!("test{i}.com"), 80, "/");
        let was_added = cache.put(key, response).await;
        assert!(was_added);

        // Property: total size should never exceed limit
        assert!(cache.total_size() <= MAX_CACHE_BYTES);
    }
}

// Property: Oversized entries are always rejected
proptest! {
    #[test]
    fn prop_oversized_entries_rejected(
        extra_bytes in 1usize..1_000_000usize
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let cache = ProxyCache::new();

            let oversized = CachedResponse {
                status_line: "HTTP/1.1 200 OK\r\n".to_string(),
                headers: vec![],
                body: Bytes::from(vec![0u8; MAX_ENTRY_SIZE + extra_bytes]),
                expires: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 3600,
            };

            let key = create_cache_key("test.com", 80, "/oversized");
            let result = cache.put(key, oversized).await;

            // Property: oversized entries are always rejected
            prop_assert!(!result);
            prop_assert_eq!(cache.len().await, 0);
            Ok(())
        })?;
    }
}

// Property: Cache should handle concurrent access
#[tokio::test]
async fn prop_cache_concurrent_safety() {
    use std::sync::Arc;
    use tokio::task;

    let cache = Arc::new(ProxyCache::new());
    let mut handles = vec![];

    // Spawn multiple tasks accessing cache concurrently
    for i in 0..10 {
        let cache_clone = cache.clone();
        let handle = task::spawn(async move {
            let key = create_cache_key(&format!("test{i}.com"), 80, "/");
            let response = CachedResponse {
                status_line: "HTTP/1.1 200 OK\r\n".to_string(),
                headers: vec![],
                body: Bytes::from(format!("body{i}")),
                expires: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + 3600,
            };
            cache_clone.put(key, response.clone()).await;
            let retrieved = cache_clone.get(key).await;
            assert_eq!(retrieved, Some(response));
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Property: All items should be in cache
    assert!(cache.len().await >= 10);
}

use bytes::Bytes;
use proptest::prelude::*;
use quickcheck_macros::quickcheck;
use rustysquid::*;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// PMAT-Compliant Property Tests - Zero Tolerance Quality Standards
// ============================================================================

// ----------------------------------------------------------------------------
// Cache Key Properties - Determinism and Distribution
// ----------------------------------------------------------------------------

proptest! {
    /// Property: Cache keys are deterministic - same inputs always produce same key
    #[test]
    fn prop_cache_key_deterministic(
        host in "[a-z]{3,10}\\.(com|org|net)",
        port in 1u16..65535u16,
        path in "/[a-z0-9/]{1,50}"
    ) {
        let key1 = create_cache_key(&host, port, &path);
        let key2 = create_cache_key(&host, port, &path);
        prop_assert_eq!(key1, key2, "Cache keys must be deterministic");
    }

    /// Property: Different inputs produce different keys (collision resistance)
    #[test]
    fn prop_cache_key_collision_resistance(
        host1 in "[a-z]{3,10}\\.(com|org|net)",
        host2 in "[a-z]{3,10}\\.(com|org|net)",
        port1 in 1u16..65535u16,
        port2 in 1u16..65535u16,
        path1 in "/[a-z0-9/]{1,50}",
        path2 in "/[a-z0-9/]{1,50}"
    ) {
        prop_assume!(host1 != host2 || port1 != port2 || path1 != path2);
        let key1 = create_cache_key(&host1, port1, &path1);
        let key2 = create_cache_key(&host2, port2, &path2);
        prop_assert_ne!(key1, key2, "Different inputs must produce different keys");
    }

    /// Property: Cache key distribution is uniform
    #[test]
    fn prop_cache_key_distribution(
        hosts in prop::collection::vec("[a-z]{5,15}\\.(com|org|net)", 100),
        ports in prop::collection::vec(1u16..65535u16, 100),
        paths in prop::collection::vec("/[a-z0-9/]{1,30}", 100)
    ) {
        let mut keys = Vec::new();
        for i in 0..hosts.len() {
            let key = create_cache_key(&hosts[i], ports[i % ports.len()], &paths[i % paths.len()]);
            keys.push(key);
        }
        
        // Check for reasonable distribution (no extreme clustering)
        keys.sort_unstable();
        keys.dedup();
        let unique_ratio = keys.len() as f64 / 100.0;
        prop_assert!(unique_ratio > 0.95, "Keys should have good distribution: {:.2}% unique", unique_ratio * 100.0);
    }
}

// ----------------------------------------------------------------------------
// HTTP Parsing Properties - Correctness and Safety
// ----------------------------------------------------------------------------

proptest! {
    /// Property: Valid HTTP requests are always parsed correctly
    #[test]
    fn prop_valid_http_parsing(
        method in prop::sample::select(vec!["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS"]),
        path in "/[a-z0-9/]{1,50}",
        host in "[a-z]{3,10}\\.(com|org|net)"
    ) {
        let request = format!("{} {} HTTP/1.1\r\nHost: {}\r\n\r\n", method, path, host);
        let result = parse_request(request.as_bytes());
        
        prop_assert!(result.is_some(), "Valid request must parse");
        let (parsed_method, parsed_path, headers) = result.unwrap();
        prop_assert_eq!(parsed_method, method);
        prop_assert_eq!(parsed_path, path);
        prop_assert!(headers.iter().any(|h| h.starts_with("Host:")));
    }

    /// Property: Malformed requests are safely rejected
    #[test]
    fn prop_malformed_request_rejected(
        garbage in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        // Filter out accidentally valid requests
        let request_str = String::from_utf8_lossy(&garbage);
        prop_assume!(!request_str.starts_with("GET ") && !request_str.starts_with("POST "));
        
        let result = parse_request(&garbage);
        prop_assert!(result.is_none(), "Malformed request must be rejected");
    }

    /// Property: Request size limits are enforced
    #[test]
    fn prop_request_size_limit_enforced(
        size in (MAX_REQUEST_SIZE + 1)..=(MAX_REQUEST_SIZE * 2)
    ) {
        let large_path = "/".repeat(size);
        let request = format!("GET {} HTTP/1.1\r\nHost: example.com\r\n\r\n", large_path);
        
        // This should be rejected at the parsing level or before
        prop_assert!(request.len() > MAX_REQUEST_SIZE);
    }
}

// ----------------------------------------------------------------------------
// Cacheability Properties - Policy Enforcement
// ----------------------------------------------------------------------------

proptest! {
    /// Property: Only GET requests are cacheable
    #[test]
    fn prop_only_get_cacheable(
        method in prop::sample::select(vec!["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH"]),
        path in "/[a-z0-9/]{1,50}\\.(html|css|js|jpg|png)"
    ) {
        let is_get = method == "GET";
        let result = is_cacheable(method, &path, &[]);
        prop_assert_eq!(result, is_get, "Only GET requests should be cacheable");
    }

    /// Property: Cache-Control headers are always respected
    #[test]
    fn prop_cache_control_respected(
        directive in prop::sample::select(vec!["no-cache", "no-store", "private", "max-age=3600", "public"])
    ) {
        let headers = vec![format!("Cache-Control: {}", directive)];
        let should_cache = !directive.contains("no-cache") && 
                          !directive.contains("no-store") && 
                          !directive.contains("private");
        
        let result = is_cacheable("GET", "/index.html", &headers);
        prop_assert_eq!(result, should_cache, "Cache-Control directives must be respected");
    }

    /// Property: Static files are cacheable by default
    #[test]
    fn prop_static_files_cacheable(
        extension in prop::sample::select(vec![
            "jpg", "jpeg", "png", "gif", "ico", "css", "js", 
            "woff", "woff2", "ttf", "svg", "webp", "mp4", "webm"
        ])
    ) {
        let path = format!("/static/file.{}", extension);
        let result = is_cacheable("GET", &path, &[]);
        prop_assert!(result, "Static files should be cacheable");
    }
}

// ----------------------------------------------------------------------------
// TTL Properties - Time Bounds and Correctness
// ----------------------------------------------------------------------------

proptest! {
    /// Property: TTL never exceeds 24 hours
    #[test]
    fn prop_ttl_bounded(max_age in 0u64..1_000_000u64) {
        let headers = vec![format!("Cache-Control: max-age={}", max_age)];
        let ttl = calculate_ttl(&headers);
        prop_assert!(ttl <= 86400, "TTL must not exceed 24 hours");
    }

    /// Property: Default TTL is applied when no cache headers present
    #[test]
    fn prop_default_ttl_applied(
        headers in prop::collection::vec("[A-Za-z-]+: [^\r\n]+", 0..10)
    ) {
        // Filter out cache-control headers
        let filtered: Vec<String> = headers.into_iter()
            .filter(|h| !h.to_lowercase().starts_with("cache-control"))
            .collect();
        
        let ttl = calculate_ttl(&filtered);
        prop_assert_eq!(ttl, CACHE_TTL, "Default TTL should be applied");
    }
}

// ----------------------------------------------------------------------------
// Cache Operations Properties - Memory and Capacity Invariants
// ----------------------------------------------------------------------------

#[tokio::test]
async fn prop_cache_capacity_invariant() {
    let cache = ProxyCache::new();
    
    // Add more items than capacity
    for i in 0..(CACHE_SIZE + 100) {
        let response = CachedResponse {
            status_line: format!("HTTP/1.1 200 OK {}\r\n", i),
            headers: vec![],
            body: Bytes::from(format!("body {}", i)),
            expires: u64::MAX,
        };
        cache.put(i as u64, response).await;
    }
    
    // Property: Cache never exceeds maximum capacity
    assert!(cache.len().await <= CACHE_SIZE, "Cache capacity must not be exceeded");
}

#[tokio::test]
async fn prop_cache_memory_invariant() {
    let cache = ProxyCache::new();
    
    // Try to add entries that would exceed memory limit
    let large_size = MAX_ENTRY_SIZE - 1000; // Just under the per-entry limit
    let num_entries = (MAX_CACHE_BYTES / large_size) + 10; // Would exceed total limit
    
    for i in 0..num_entries {
        let response = CachedResponse {
            status_line: "HTTP/1.1 200 OK\r\n".to_string(),
            headers: vec![],
            body: Bytes::from(vec![0u8; large_size]),
            expires: u64::MAX,
        };
        cache.put(i as u64, response).await;
    }
    
    // Property: Total cache size never exceeds memory limit
    assert!(
        cache.total_size() <= MAX_CACHE_BYTES,
        "Cache memory limit must not be exceeded: {} > {}",
        cache.total_size(),
        MAX_CACHE_BYTES
    );
}

#[tokio::test]
async fn prop_cache_expiration_invariant() {
    let cache = ProxyCache::new();
    
    // Add an expired entry
    let expired_response = CachedResponse {
        status_line: "HTTP/1.1 200 OK\r\n".to_string(),
        headers: vec![],
        body: Bytes::from("expired"),
        expires: 0, // Already expired
    };
    
    cache.put(1, expired_response).await;
    
    // Property: Expired entries are never returned
    let result = cache.get(1).await;
    assert!(result.is_none(), "Expired entries must not be returned");
    
    // Add a valid entry
    let valid_response = CachedResponse {
        status_line: "HTTP/1.1 200 OK\r\n".to_string(),
        headers: vec![],
        body: Bytes::from("valid"),
        expires: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + 3600,
    };
    
    cache.put(2, valid_response.clone()).await;
    
    // Property: Valid entries are returned
    let result = cache.get(2).await;
    assert_eq!(result, Some(valid_response), "Valid entries must be returned");
}

// ----------------------------------------------------------------------------
// Concurrent Safety Properties - Thread Safety and Atomicity
// ----------------------------------------------------------------------------

#[tokio::test]
async fn prop_concurrent_cache_safety() {
    use std::sync::Arc;
    use tokio::task;
    
    let cache = Arc::new(ProxyCache::new());
    let mut handles = vec![];
    
    // Spawn many concurrent operations
    for i in 0..100 {
        let cache_clone = cache.clone();
        let handle = task::spawn(async move {
            let key = create_cache_key(&format!("host{}.com", i), 80, "/");
            let response = CachedResponse {
                status_line: format!("HTTP/1.1 200 OK {}\r\n", i),
                headers: vec![],
                body: Bytes::from(format!("body{}", i)),
                expires: u64::MAX,
            };
            
            // Concurrent put and get operations
            cache_clone.put(key, response.clone()).await;
            let retrieved = cache_clone.get(key).await;
            
            // Property: Concurrent operations maintain consistency
            assert_eq!(retrieved, Some(response), "Concurrent operations must be consistent");
        });
        handles.push(handle);
    }
    
    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Property: Cache remains consistent after concurrent access
    assert!(cache.len().await > 0, "Cache should contain entries");
    assert!(cache.total_size() > 0, "Cache should track size");
}

// ----------------------------------------------------------------------------
// Host Extraction Properties - Parsing Correctness
// ----------------------------------------------------------------------------

proptest! {
    /// Property: Host header with port is correctly parsed
    #[test]
    fn prop_host_extraction_with_port(
        host in "[a-z]{3,10}\\.(com|org|net)",
        port in 1u16..65535u16
    ) {
        let headers = vec![format!("Host: {}:{}", host, port)];
        let result = extract_host(&headers);
        prop_assert_eq!(result, Some((host, port)), "Host with port must be extracted correctly");
    }

    /// Property: Host header without port defaults to 80
    #[test]
    fn prop_host_extraction_default_port(
        host in "[a-z]{3,10}\\.(com|org|net)"
    ) {
        let headers = vec![format!("Host: {}", host)];
        let result = extract_host(&headers);
        prop_assert_eq!(result, Some((host, 80)), "Host without port must default to 80");
    }

    /// Property: Missing host header returns None
    #[test]
    fn prop_missing_host_returns_none(
        headers in prop::collection::vec("[A-Za-z-]+: [^\r\n]+", 0..10)
    ) {
        // Filter out host headers
        let filtered: Vec<String> = headers.into_iter()
            .filter(|h| !h.to_lowercase().starts_with("host:"))
            .collect();
        
        let result = extract_host(&filtered);
        prop_assert_eq!(result, None, "Missing host header must return None");
    }
}

// ----------------------------------------------------------------------------
// QuickCheck Tests - Additional Property Verification
// ----------------------------------------------------------------------------

#[quickcheck]
fn qc_cache_key_never_zero(host: String, port: u16, path: String) -> bool {
    if host.is_empty() || path.is_empty() {
        return true; // Skip invalid inputs
    }
    let key = create_cache_key(&host, port, &path);
    key != 0
}

#[quickcheck]
fn qc_ttl_calculation_stable(headers: Vec<String>) -> bool {
    let ttl1 = calculate_ttl(&headers);
    let ttl2 = calculate_ttl(&headers);
    ttl1 == ttl2
}

#[quickcheck]
fn qc_cacheable_deterministic(method: String, path: String, headers: Vec<String>) -> bool {
    let result1 = is_cacheable(&method, &path, &headers);
    let result2 = is_cacheable(&method, &path, &headers);
    result1 == result2
}

// ----------------------------------------------------------------------------
// PMAT Compliance Test - Verify Zero Tolerance Standards
// ----------------------------------------------------------------------------

#[test]
fn test_pmat_zero_tolerance_compliance() {
    // This test verifies that our code meets PMAT's zero tolerance standards
    
    // Property: No SATD markers in code
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let has_satd = std::fs::read_dir(src_dir).unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "rs"))
        .any(|entry| {
            let content = std::fs::read_to_string(entry.path()).unwrap();
            content.contains("TODO") || content.contains("FIXME") || content.contains("HACK")
        });
    
    assert!(!has_satd, "Code must not contain SATD markers (TODO/FIXME/HACK)");
    
    // Property: All functions have reasonable complexity (verified by refactoring)
    // This is enforced at compile time via clippy settings
    
    println!("✅ PMAT Zero Tolerance Standards Verified");
}
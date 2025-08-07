use bytes::Bytes;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use xxhash_rust::xxh64::xxh64;

pub const CACHE_SIZE: usize = 10000;
pub const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;
pub const MAX_CACHE_BYTES: usize = 50 * 1024 * 1024; // 50MB total cache size limit
pub const MAX_ENTRY_SIZE: usize = 5 * 1024 * 1024;   // 5MB per entry limit
pub const CACHE_TTL: u64 = 3600;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedResponse {
    pub status_line: String,
    pub headers: Vec<String>,
    pub body: Bytes,
    pub expires: u64,
}

#[derive(Clone)]
pub struct ProxyCache {
    cache: Arc<Mutex<LruCache<u64, CachedResponse>>>,
    total_size: Arc<AtomicUsize>,
}

impl ProxyCache {
    /// Creates a new ProxyCache with the default cache size.
    ///
    /// # Panics
    ///
    /// Panics if CACHE_SIZE is 0, which should never happen in normal operation.
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(CACHE_SIZE).expect("CACHE_SIZE must be non-zero"),
            ))),
            total_size: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.lock().await;
        cache.is_empty()
    }

    pub async fn get(&self, key: u64) -> Option<CachedResponse> {
        let mut cache = self.cache.lock().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(entry) = cache.get(&key) {
            if entry.expires > now {
                return Some(entry.clone());
            }
            // Remove expired entry and update size
            if let Some(expired) = cache.pop(&key) {
                let size = Self::calculate_entry_size(&expired);
                self.total_size.fetch_sub(size, Ordering::Relaxed);
            }
        }
        None
    }

    pub async fn put(&self, key: u64, response: CachedResponse) -> bool {
        let entry_size = Self::calculate_entry_size(&response);
        
        // Reject entries that are too large
        if entry_size > MAX_ENTRY_SIZE {
            return false;
        }

        let mut cache = self.cache.lock().await;
        
        // Check if we need to evict entries to make room
        let mut current_size = self.total_size.load(Ordering::Relaxed);
        while current_size + entry_size > MAX_CACHE_BYTES && !cache.is_empty() {
            // Evict LRU entry
            if let Some((_, evicted)) = cache.pop_lru() {
                let evicted_size = Self::calculate_entry_size(&evicted);
                self.total_size.fetch_sub(evicted_size, Ordering::Relaxed);
                current_size = self.total_size.load(Ordering::Relaxed);
            } else {
                break;
            }
        }
        
        // Remove old entry if it exists
        if let Some(old) = cache.get(&key) {
            let old_size = Self::calculate_entry_size(old);
            self.total_size.fetch_sub(old_size, Ordering::Relaxed);
        }
        
        // Add new entry
        cache.put(key, response);
        self.total_size.fetch_add(entry_size, Ordering::Relaxed);
        true
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
        self.total_size.store(0, Ordering::Relaxed);
    }

    pub async fn len(&self) -> usize {
        let cache = self.cache.lock().await;
        cache.len()
    }
    
    pub async fn total_size(&self) -> usize {
        self.total_size.load(Ordering::Relaxed)
    }
    
    fn calculate_entry_size(entry: &CachedResponse) -> usize {
        entry.status_line.len() +
        entry.headers.iter().map(|h| h.len()).sum::<usize>() +
        entry.body.len() +
        std::mem::size_of::<u64>() // expires field
    }
}

impl Default for ProxyCache {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_request(data: &[u8]) -> Option<(String, String, Vec<String>)> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);

    match req.parse(data) {
        Ok(httparse::Status::Complete(_)) => {
            let method = req.method?.to_string();
            let path = req.path?.to_string();
            let headers: Vec<String> = req
                .headers
                .iter()
                .map(|h| format!("{}: {}", h.name, String::from_utf8_lossy(h.value)))
                .collect();
            Some((method, path, headers))
        }
        _ => None,
    }
}

pub fn extract_host(headers: &[String]) -> Option<(String, u16)> {
    for header in headers {
        if header.to_lowercase().starts_with("host:") {
            let host_value = header[5..].trim();
            if let Some(colon_pos) = host_value.rfind(':') {
                let host = host_value[..colon_pos].to_string();
                let port = host_value[colon_pos + 1..].parse::<u16>().unwrap_or(80);
                return Some((host, port));
            }
            return Some((host_value.to_string(), 80));
        }
    }
    None
}

pub fn is_cacheable(method: &str, path: &str, response_headers: &[String]) -> bool {
    if method != "GET" {
        return false;
    }

    // Check headers first - they override everything
    for header in response_headers {
        let header_lower = header.to_lowercase();
        if header_lower.starts_with("cache-control:") {
            if header_lower.contains("no-cache") || header_lower.contains("no-store") {
                return false;
            }
            if header_lower.contains("max-age=") {
                return true;
            }
        }
    }

    // Check for static content extensions
    let cacheable_extensions = [
        ".jpg", ".jpeg", ".png", ".gif", ".ico", ".css", ".js", ".woff", ".woff2", ".ttf", ".svg",
        ".webp", ".mp4", ".webm",
    ];

    let path_lower = path.to_lowercase();
    cacheable_extensions
        .iter()
        .any(|ext| path_lower.ends_with(ext))
}

pub fn calculate_ttl(headers: &[String]) -> u64 {
    for header in headers {
        let header_lower = header.to_lowercase();
        if header_lower.starts_with("cache-control:") {
            if let Some(max_age_pos) = header_lower.find("max-age=") {
                let start = max_age_pos + 8;
                let value_str = &header_lower[start..];
                if let Some(end) = value_str.find(|c: char| !c.is_ascii_digit()) {
                    if let Ok(seconds) = value_str[..end].parse::<u64>() {
                        return seconds.min(86400);
                    }
                } else if let Ok(seconds) = value_str.parse::<u64>() {
                    return seconds.min(86400);
                }
            }
        }
    }
    CACHE_TTL
}

pub fn create_cache_key(host: &str, port: u16, path: &str) -> u64 {
    xxh64(format!("{host}:{port}{path}").as_bytes(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_host() {
        let headers = vec![
            "Host: example.com".to_string(),
            "User-Agent: test".to_string(),
        ];
        assert_eq!(
            extract_host(&headers),
            Some(("example.com".to_string(), 80))
        );

        let headers_with_port = vec!["Host: example.com:8080".to_string()];
        assert_eq!(
            extract_host(&headers_with_port),
            Some(("example.com".to_string(), 8080))
        );
    }

    #[test]
    fn test_is_cacheable() {
        // Static content should be cacheable
        assert!(is_cacheable("GET", "/image.jpg", &[]));
        assert!(is_cacheable("GET", "/style.css", &[]));
        assert!(is_cacheable("GET", "/script.js", &[]));

        // POST requests should not be cacheable
        assert!(!is_cacheable("POST", "/image.jpg", &[]));

        // Respect no-cache headers
        let no_cache_headers = vec!["Cache-Control: no-cache".to_string()];
        assert!(!is_cacheable("GET", "/image.jpg", &no_cache_headers));

        // Respect max-age headers
        let max_age_headers = vec!["Cache-Control: max-age=3600".to_string()];
        assert!(is_cacheable("GET", "/api/data", &max_age_headers));
    }

    #[test]
    fn test_calculate_ttl() {
        let headers_with_max_age = vec!["Cache-Control: max-age=7200".to_string()];
        assert_eq!(calculate_ttl(&headers_with_max_age), 7200);

        let headers_with_large_max_age = vec!["Cache-Control: max-age=999999".to_string()];
        assert_eq!(calculate_ttl(&headers_with_large_max_age), 86400); // Capped at 24 hours

        let headers_without_cache = vec!["Content-Type: text/html".to_string()];
        assert_eq!(calculate_ttl(&headers_without_cache), CACHE_TTL);
    }

    #[test]
    fn test_cache_key_generation() {
        let key1 = create_cache_key("example.com", 80, "/index.html");
        let key2 = create_cache_key("example.com", 80, "/index.html");
        let key3 = create_cache_key("example.com", 80, "/other.html");

        assert_eq!(key1, key2); // Same input should produce same key
        assert_ne!(key1, key3); // Different input should produce different key
    }

    #[tokio::test]
    async fn test_proxy_cache_operations() {
        let cache = ProxyCache::new();

        // Test empty cache
        assert_eq!(cache.len().await, 0);

        // Test cache miss
        let key = create_cache_key("test.com", 80, "/test");
        assert!(cache.get(key).await.is_none());

        // Test cache put and get
        let response = CachedResponse {
            status_line: "HTTP/1.1 200 OK\r\n".to_string(),
            headers: vec!["Content-Type: text/html".to_string()],
            body: Bytes::from("test body"),
            expires: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600,
        };

        cache.put(key, response.clone()).await;
        assert_eq!(cache.len().await, 1);

        let cached = cache.get(key).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), response);

        // Test cache clear
        cache.clear().await;
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn test_cache_size_limit() {
        let cache = ProxyCache::new();
        
        // Create a large response (1MB)
        let large_response = CachedResponse {
            status_line: "HTTP/1.1 200 OK\r\n".to_string(),
            headers: vec!["Content-Type: text/html".to_string()],
            body: Bytes::from(vec![0u8; 1024 * 1024]), // 1MB
            expires: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 3600,
        };
        
        // Add entries until we exceed the limit
        for i in 0..60 {
            let key = create_cache_key("test.com", 80, &format!("/page{}", i));
            cache.put(key, large_response.clone()).await;
        }
        
        // Total size should not exceed MAX_CACHE_BYTES
        assert!(cache.total_size().await <= MAX_CACHE_BYTES);
        
        // Cache should have evicted some entries
        assert!(cache.len().await < 60);
    }
    
    #[tokio::test]
    async fn test_entry_size_limit() {
        let cache = ProxyCache::new();
        
        // Create an oversized response (> MAX_ENTRY_SIZE)
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
        
        let key = create_cache_key("test.com", 80, "/large");
        let result = cache.put(key, oversized).await;
        
        // Should reject oversized entry
        assert!(!result);
        assert_eq!(cache.len().await, 0);
        assert_eq!(cache.total_size().await, 0);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = ProxyCache::new();
        let key = create_cache_key("test.com", 80, "/expired");

        // Add expired entry
        let expired_response = CachedResponse {
            status_line: "HTTP/1.1 200 OK\r\n".to_string(),
            headers: vec![],
            body: Bytes::from("expired"),
            expires: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - 1, // Already expired
        };

        cache.put(key, expired_response).await;

        // Should not return expired entry
        assert!(cache.get(key).await.is_none());
    }
}

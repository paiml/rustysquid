use bytes::{Bytes, BytesMut};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::timeout;
use xxhash_rust::xxh64::xxh64;

const PROXY_PORT: u16 = 3128;
const CACHE_SIZE: usize = 10000; // Number of cached responses
const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024; // 10MB max cached response
const CACHE_TTL: u64 = 3600; // 1 hour default TTL
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
struct CachedResponse {
    status_line: String,
    headers: Vec<String>,
    body: Bytes,
    expires: u64,
}

#[derive(Clone)]
struct ProxyCache {
    cache: Arc<Mutex<LruCache<u64, CachedResponse>>>,
}

impl ProxyCache {
    fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(CACHE_SIZE).unwrap(),
            ))),
        }
    }

    async fn get(&self, key: u64) -> Option<CachedResponse> {
        let mut cache = self.cache.lock().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(entry) = cache.get(&key) {
            if entry.expires > now {
                return Some(entry.clone());
            } else {
                cache.pop(&key);
            }
        }
        None
    }

    async fn put(&self, key: u64, response: CachedResponse) {
        let mut cache = self.cache.lock().await;
        cache.put(key, response);
    }
}

fn parse_request(data: &[u8]) -> Option<(String, String, Vec<String>)> {
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

fn extract_host(headers: &[String]) -> Option<(String, u16)> {
    for header in headers {
        if header.to_lowercase().starts_with("host:") {
            let host_value = header[5..].trim();
            if let Some(colon_pos) = host_value.rfind(':') {
                let host = host_value[..colon_pos].to_string();
                let port = host_value[colon_pos + 1..].parse::<u16>().unwrap_or(80);
                return Some((host, port));
            } else {
                return Some((host_value.to_string(), 80));
            }
        }
    }
    None
}

fn is_cacheable(method: &str, path: &str, response_headers: &[String]) -> bool {
    // Only cache GET requests
    if method != "GET" {
        return false;
    }

    // Check for static content extensions
    let cacheable_extensions = [
        ".jpg", ".jpeg", ".png", ".gif", ".ico", ".css", ".js", ".woff", ".woff2", ".ttf", ".svg",
        ".webp", ".mp4", ".webm",
    ];

    let path_lower = path.to_lowercase();
    let is_static = cacheable_extensions
        .iter()
        .any(|ext| path_lower.ends_with(ext));

    if is_static {
        return true;
    }

    // Check Cache-Control headers
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

    false
}

fn calculate_ttl(headers: &[String]) -> u64 {
    for header in headers {
        let header_lower = header.to_lowercase();
        if header_lower.starts_with("cache-control:") {
            if let Some(max_age_pos) = header_lower.find("max-age=") {
                let start = max_age_pos + 8;
                let value_str = &header_lower[start..];
                if let Some(end) = value_str.find(|c: char| !c.is_ascii_digit()) {
                    if let Ok(seconds) = value_str[..end].parse::<u64>() {
                        return seconds.min(86400); // Cap at 24 hours
                    }
                } else if let Ok(seconds) = value_str.parse::<u64>() {
                    return seconds.min(86400);
                }
            }
        }
    }
    CACHE_TTL
}

async fn handle_client(mut client: TcpStream, cache: ProxyCache) {
    let mut buffer = BytesMut::with_capacity(8192);

    // Read request
    match timeout(CONNECTION_TIMEOUT, client.read_buf(&mut buffer)).await {
        Ok(Ok(0)) | Ok(Err(_)) | Err(_) => return,
        Ok(Ok(_)) => {}
    }

    // Parse request
    let (method, path, headers) = match parse_request(&buffer) {
        Some(parsed) => parsed,
        None => {
            let _ = client.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
            return;
        }
    };

    // Extract host
    let (host, port) = match extract_host(&headers) {
        Some(host_info) => host_info,
        None => {
            let _ = client.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
            return;
        }
    };

    // Create cache key
    let cache_key = xxh64(format!("{}:{}{}", host, port, path).as_bytes(), 0);

    // Check cache for GET requests
    if method == "GET" {
        if let Some(cached) = cache.get(cache_key).await {
            println!("CACHE HIT: {}{}", host, path);
            let _ = client.write_all(cached.status_line.as_bytes()).await;
            for header in &cached.headers {
                let _ = client.write_all(header.as_bytes()).await;
                let _ = client.write_all(b"\r\n").await;
            }
            let _ = client.write_all(b"\r\n").await;
            let _ = client.write_all(&cached.body).await;
            return;
        }
    }

    println!("CACHE MISS: {}{}", host, path);

    // Connect to upstream server
    let upstream = match timeout(
        Duration::from_secs(10),
        TcpStream::connect((host.as_str(), port)),
    )
    .await
    {
        Ok(Ok(stream)) => stream,
        _ => {
            let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
            return;
        }
    };

    let (mut upstream_read, mut upstream_write) = upstream.into_split();

    // Forward request to upstream
    let _ = upstream_write.write_all(&buffer).await;

    // Read response
    let mut response_buffer = BytesMut::with_capacity(8192);
    let mut total_size = 0;

    loop {
        match timeout(
            CONNECTION_TIMEOUT,
            upstream_read.read_buf(&mut response_buffer),
        )
        .await
        {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                total_size += n;
                if total_size > MAX_RESPONSE_SIZE {
                    // Response too large, stop caching but continue forwarding
                    let _ = client.write_all(&response_buffer).await;
                    tokio::io::copy(&mut upstream_read, &mut client).await.ok();
                    return;
                }
            }
            _ => break,
        }
    }

    // Parse response for caching
    let response_data = response_buffer.freeze();

    // Forward response to client
    let _ = client.write_all(&response_data).await;

    // Parse and cache response if applicable
    if method == "GET" && total_size <= MAX_RESPONSE_SIZE {
        if let Some((status_line, resp_headers, body)) = parse_response(&response_data) {
            if is_cacheable(&method, &path, &resp_headers) {
                let ttl = calculate_ttl(&resp_headers);
                let expires = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    + ttl;

                let cached_response = CachedResponse {
                    status_line,
                    headers: resp_headers,
                    body: Bytes::copy_from_slice(body),
                    expires,
                };

                cache.put(cache_key, cached_response).await;
                println!("CACHED: {}{} (TTL: {}s)", host, path, ttl);
            }
        }
    }
}

fn parse_response(data: &[u8]) -> Option<(String, Vec<String>, &[u8])> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut response = httparse::Response::new(&mut headers);

    match response.parse(data) {
        Ok(httparse::Status::Complete(header_len)) => {
            let status = response.code?;
            let status_line = format!(
                "HTTP/1.1 {} {}\r\n",
                status,
                response.reason.unwrap_or("OK")
            );

            let headers: Vec<String> = response
                .headers
                .iter()
                .map(|h| format!("{}: {}", h.name, String::from_utf8_lossy(h.value)))
                .collect();

            let body = &data[header_len..];
            Some((status_line, headers, body))
        }
        _ => None,
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("RustySquid v0.1.0 - Minimal HTTP Cache Proxy");
    println!("Listening on port {}", PROXY_PORT);
    println!("Cache size: {} entries", CACHE_SIZE);
    println!(
        "Max cached response: {} MB",
        MAX_RESPONSE_SIZE / 1024 / 1024
    );

    let cache = ProxyCache::new();
    let listener = TcpListener::bind(format!("0.0.0.0:{}", PROXY_PORT))
        .await
        .expect("Failed to bind to port");

    loop {
        match listener.accept().await {
            Ok((client, _addr)) => {
                let cache_clone = cache.clone();
                tokio::spawn(async move {
                    handle_client(client, cache_clone).await;
                });
            }
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
            }
        }
    }
}

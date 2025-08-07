use bytes::{Bytes, BytesMut};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

// Import from lib
use rustysquid::{
    CachedResponse, ProxyCache, 
    create_cache_key, is_cacheable, calculate_ttl,
    parse_request, extract_host,
    MAX_RESPONSE_SIZE, CACHE_SIZE
};

const PROXY_PORT: u16 = 3128;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);





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
    let cache_key = create_cache_key(&host, port, &path);

    // Check cache for GET requests
    if method == "GET" {
        if let Some(cached) = cache.get(cache_key).await {
            info!("CACHE HIT: {}{}", host, path);
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

    debug!("CACHE MISS: {}{}", host, path);

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
                    .unwrap_or_default()
                    .as_secs()
                    + ttl;

                let cached_response = CachedResponse {
                    status_line,
                    headers: resp_headers,
                    body: Bytes::copy_from_slice(body),
                    expires,
                };

                if cache.put(cache_key, cached_response).await {
                    info!("CACHED: {}{} (TTL: {}s)", host, path, ttl);
                } else {
                    warn!("CACHE REJECTED (too large): {}{}", host, path);
                }
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rustysquid=info".parse()?)
        )
        .init();

    info!("RustySquid v0.1.0 - Minimal HTTP Cache Proxy");
    info!("Listening on port {}", PROXY_PORT);
    info!("Cache size: {} entries", CACHE_SIZE);
    info!(
        "Max cached response: {} MB",
        MAX_RESPONSE_SIZE / 1024 / 1024
    );

    let cache = ProxyCache::new();
    let listener = TcpListener::bind(format!("0.0.0.0:{}", PROXY_PORT)).await?;

    loop {
        match listener.accept().await {
            Ok((client, _addr)) => {
                let cache_clone = cache.clone();
                tokio::spawn(async move {
                    handle_client(client, cache_clone).await;
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

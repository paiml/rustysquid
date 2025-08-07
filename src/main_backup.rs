use bytes::{Bytes, BytesMut};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

// Import from lib
use rustysquid::{
    calculate_ttl, create_cache_key, extract_host, is_cacheable, parse_request, CachedResponse,
    ProxyCache, CACHE_SIZE, MAX_CONNECTIONS, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE,
};

const PROXY_PORT: u16 = 3128;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);

async fn handle_client(
    mut client: TcpStream,
    cache: ProxyCache,
    _active_connections: Arc<AtomicUsize>,
) {
    let mut buffer = BytesMut::with_capacity(8192);
    let mut total_read = 0;

    // Read request with size limit
    loop {
        match timeout(CONNECTION_TIMEOUT, client.read_buf(&mut buffer)).await {
            Ok(Ok(0)) => break, // End of stream
            Ok(Ok(n)) => {
                total_read += n;
                if total_read > MAX_REQUEST_SIZE {
                    warn!("Request too large ({} bytes), rejecting", total_read);
                    if let Err(e) = client
                        .write_all(b"HTTP/1.1 413 Request Entity Too Large\r\n\r\n")
                        .await
                    {
                        debug!("Failed to send 413 response: {}", e);
                    }
                    return;
                }
                // Check if we have complete headers
                if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Ok(Err(_)) | Err(_) => return,
        }
    }

    if buffer.is_empty() {
        return;
    }

    // Parse request
    let Some((method, path, headers)) = parse_request(&buffer) else {
        if let Err(e) = client.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await {
            debug!("Failed to send 400 response: {}", e);
        }
        return;
    };

    // Extract host
    let Some((host, port)) = extract_host(&headers) else {
        if let Err(e) = client.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await {
            debug!("Failed to send 400 response: {}", e);
        }
        return;
    };

    // Create cache key
    let cache_key = create_cache_key(&host, port, &path);

    // Check cache for GET requests
    if method == "GET" {
        if let Some(cached) = cache.get(cache_key).await {
            info!("CACHE HIT: {}{}", host, path);
            if let Err(e) = client.write_all(cached.status_line.as_bytes()).await {
                debug!("Failed to write cached status line: {}", e);
                return;
            }
            for header in &cached.headers {
                if let Err(e) = client.write_all(header.as_bytes()).await {
                    debug!("Failed to write cached header: {}", e);
                    return;
                }
                if let Err(e) = client.write_all(b"\r\n").await {
                    debug!("Failed to write CRLF: {}", e);
                    return;
                }
            }
            if let Err(e) = client.write_all(b"\r\n").await {
                debug!("Failed to write final CRLF: {}", e);
                return;
            }
            if let Err(e) = client.write_all(&cached.body).await {
                debug!("Failed to write cached body: {}", e);
                return;
            }
            return;
        }
    }

    debug!("CACHE MISS: {}{}", host, path);

    // Connect to upstream server
    let Ok(Ok(upstream)) = timeout(
        Duration::from_secs(10),
        TcpStream::connect((host.as_str(), port)),
    )
    .await
    else {
        if let Err(e) = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await {
            debug!("Failed to send 502 response: {}", e);
        }
        return;
    };

    let (mut upstream_read, mut upstream_write) = upstream.into_split();

    // Forward request to upstream
    if let Err(e) = upstream_write.write_all(&buffer).await {
        debug!("Failed to forward request to upstream: {}", e);
        if let Err(e) = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await {
            debug!("Failed to send 502 response: {}", e);
        }
        return;
    }

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
                    if let Err(e) = client.write_all(&response_buffer).await {
                        debug!("Failed to write oversized response: {}", e);
                        return;
                    }
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
    if let Err(e) = client.write_all(&response_data).await {
        debug!("Failed to forward response to client: {}", e);
    }

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
                .add_directive("rustysquid=info".parse()?),
        )
        .init();

    info!("RustySquid v0.1.0 - Minimal HTTP Cache Proxy");
    info!("Listening on port {}", PROXY_PORT);
    info!("Cache size: {} entries", CACHE_SIZE);
    info!("Max connections: {}", MAX_CONNECTIONS);
    info!(
        "Max cached response: {} MB",
        MAX_RESPONSE_SIZE / 1024 / 1024
    );

    let cache = ProxyCache::new();
    let listener = TcpListener::bind(format!("0.0.0.0:{PROXY_PORT}")).await?;
    let active_connections = Arc::new(AtomicUsize::new(0));

    // Setup graceful shutdown
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    loop {
        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, initiating graceful shutdown");
                break;
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, initiating graceful shutdown");
                break;
            }
            result = listener.accept() => {
                match result {
            Ok((client, addr)) => {
                let current = active_connections.load(Ordering::Relaxed);
                if current >= MAX_CONNECTIONS {
                    warn!("Connection limit reached ({}), rejecting {}", MAX_CONNECTIONS, addr);
                    // Send error response and close
                    match client.try_write(b"HTTP/1.1 503 Service Unavailable\r\n\r\n") {
                        Ok(_) => {},
                        Err(e) => debug!("Failed to send 503 response: {}", e),
                    }
                    drop(client);
                    continue;
                }

                // Increment connection count
                active_connections.fetch_add(1, Ordering::Relaxed);
                debug!("Accepted connection from {} (active: {})", addr, current + 1);

                let cache_clone = cache.clone();
                let connections_clone = active_connections.clone();

                tokio::spawn(async move {
                    handle_client(client, cache_clone, connections_clone.clone()).await;
                    // Decrement on completion
                    let remaining = connections_clone.fetch_sub(1, Ordering::Relaxed) - 1;
                    debug!("Connection closed (active: {})", remaining);
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
                }
            }
        }
    }

    // Wait for active connections to finish
    info!(
        "Waiting for {} active connections to close",
        active_connections.load(Ordering::Relaxed)
    );
    while active_connections.load(Ordering::Relaxed) > 0 {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    info!("All connections closed, shutting down");

    Ok(())
}

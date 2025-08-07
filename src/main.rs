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
    connection_pool::ConnectionPool,
};

const PROXY_PORT: u16 = 3128;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);

// Refactored with reduced complexity - each function has cyclomatic complexity <= 10

/// Read HTTP request from client with size limits
async fn read_client_request(client: &mut TcpStream) -> Result<BytesMut, &'static str> {
    let mut buffer = BytesMut::with_capacity(8192);
    let mut total_read = 0;

    loop {
        match timeout(CONNECTION_TIMEOUT, client.read_buf(&mut buffer)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                total_read += n;
                if total_read > MAX_REQUEST_SIZE {
                    return Err("Request too large");
                }
                if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            _ => return Err("Read timeout or error"),
        }
    }
    
    Ok(buffer)
}

/// Send error response to client
async fn send_error_response(client: &mut TcpStream, status: &[u8]) {
    if let Err(e) = client.write_all(status).await {
        debug!("Failed to send error response: {}", e);
    }
}

/// Parse and validate HTTP request
fn validate_request(buffer: &[u8]) -> Result<(String, String, Vec<String>), &'static str> {
    let (method, path, headers) = parse_request(buffer).ok_or("Invalid request")?;
    let (host, port) = extract_host(&headers).ok_or("Missing host header")?;
    Ok((method, format!("{}:{}{}", host, port, path), headers))
}

/// Serve response from cache
async fn serve_cached_response(
    client: &mut TcpStream,
    cached: Arc<CachedResponse>,
) -> Result<(), &'static str> {
    client.write_all(cached.status_line.as_bytes()).await
        .map_err(|_| "Failed to write status")?;
    
    for header in &cached.headers {
        client.write_all(header.as_bytes()).await
            .map_err(|_| "Failed to write header")?;
        client.write_all(b"\r\n").await
            .map_err(|_| "Failed to write CRLF")?;
    }
    
    client.write_all(b"\r\n").await
        .map_err(|_| "Failed to write final CRLF")?;
    client.write_all(&cached.body).await
        .map_err(|_| "Failed to write body")?;
    
    Ok(())
}


/// Forward request to upstream and get response
async fn forward_to_upstream(
    upstream: &mut TcpStream,
    request: &[u8],
) -> Result<BytesMut, &'static str> {
    let (mut upstream_read, mut upstream_write) = upstream.split();
    
    // Send request
    upstream_write.write_all(request).await
        .map_err(|_| "Failed to forward request")?;
    
    // Read response
    let mut response_buffer = BytesMut::with_capacity(8192);
    let mut total_size = 0;
    
    loop {
        match timeout(CONNECTION_TIMEOUT, upstream_read.read_buf(&mut response_buffer)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                total_size += n;
                if total_size > MAX_RESPONSE_SIZE {
                    return Err("Response too large");
                }
            }
            _ => break,
        }
    }
    
    Ok(response_buffer)
}

/// Parse response headers for caching decision
fn parse_response_for_cache(
    response: &[u8],
    method: &str,
    path: &str,
) -> Option<CachedResponse> {
    let mut headers_end = 0;
    for i in 0..response.len().saturating_sub(3) {
        if &response[i..i + 4] == b"\r\n\r\n" {
            headers_end = i + 4;
            break;
        }
    }
    
    if headers_end == 0 {
        return None;
    }
    
    let headers_bytes = &response[..headers_end];
    let body = &response[headers_end..];
    
    // Parse status line and headers
    let headers_str = String::from_utf8_lossy(headers_bytes);
    let lines: Vec<String> = headers_str.lines().map(|s| s.to_string()).collect();
    
    if lines.is_empty() {
        return None;
    }
    
    let status_line = format!("{}\r\n", lines[0]);
    let headers = lines[1..]
        .iter()
        .filter(|h| !h.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    
    // Check if cacheable
    if !is_cacheable(method, path, &headers) {
        return None;
    }
    
    // Calculate TTL
    let ttl = calculate_ttl(&headers);
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() + ttl;
    
    Some(CachedResponse {
        status_line,
        headers,
        body: Bytes::copy_from_slice(body),
        expires,
    })
}

/// Main client handler with reduced complexity
async fn handle_client(
    mut client: TcpStream,
    cache: ProxyCache,
    pool: ConnectionPool,
    _active_connections: Arc<AtomicUsize>,
) {
    // Step 1: Read request
    let buffer = match read_client_request(&mut client).await {
        Ok(buf) => buf,
        Err(e) => {
            warn!("Failed to read request: {}", e);
            if e == "Request too large" {
                send_error_response(&mut client, b"HTTP/1.1 413 Request Entity Too Large\r\n\r\n").await;
            }
            return;
        }
    };
    
    // Step 2: Parse and validate request
    let (method, full_path, _headers) = match validate_request(&buffer) {
        Ok(result) => result,
        Err(e) => {
            debug!("Invalid request: {}", e);
            send_error_response(&mut client, b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
            return;
        }
    };
    
    // Extract host and path from full_path
    let parts: Vec<&str> = full_path.splitn(2, '/').collect();
    let host_port = parts[0];
    let path = format!("/{}", parts.get(1).unwrap_or(&""));
    let host_parts: Vec<&str> = host_port.split(':').collect();
    let host = host_parts[0];
    let port: u16 = host_parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(80);
    
    // Step 3: Check cache for GET requests
    let cache_key = create_cache_key(host, port, &path);
    
    if method == "GET" {
        if let Some(cached) = cache.get(cache_key).await {
            info!("CACHE HIT: {}{}", host, path);
            if serve_cached_response(&mut client, cached).await.is_err() {
                debug!("Failed to serve cached response");
            }
            return;
        }
    }
    
    debug!("CACHE MISS: {}{}", host, path);
    
    // Step 4: Get connection from pool
    let mut upstream = match pool.get_connection(host, port).await {
        Ok(stream) => stream,
        Err(e) => {
            debug!("Failed to get connection from pool: {}", e);
            send_error_response(&mut client, b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
            return;
        }
    };
    
    // Step 5: Forward request and get response
    let response_buffer = match forward_to_upstream(&mut upstream, &buffer).await {
        Ok(resp) => resp,
        Err(e) => {
            debug!("Failed to get upstream response: {}", e);
            send_error_response(&mut client, b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
            return;
        }
    };
    
    // Step 6: Send response to client
    if let Err(e) = client.write_all(&response_buffer).await {
        debug!("Failed to send response to client: {}", e);
        return;
    }
    
    // Step 7: Return connection to pool
    pool.return_connection(host.to_string(), port, upstream).await;
    
    // Step 8: Cache response if applicable
    if let Some(cached_response) = parse_response_for_cache(&response_buffer, &method, &path) {
        let ttl = cached_response.expires - SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        if cache.put(cache_key, cached_response).await {
            info!("CACHED: {}{} (TTL: {}s)", host, path, ttl);
        }
    }
}

/// Connection acceptor with proper connection limiting
async fn accept_connections(listener: TcpListener, cache: ProxyCache, pool: ConnectionPool) {
    let active_connections = Arc::new(AtomicUsize::new(0));
    
    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                continue;
            }
        };
        
        // Check connection limit
        if active_connections.load(Ordering::Relaxed) >= MAX_CONNECTIONS {
            debug!("Connection limit reached, rejecting {}", addr);
            drop(stream);
            continue;
        }
        
        // Handle client
        let cache_clone = cache.clone();
        let pool_clone = pool.clone();
        let connections = Arc::clone(&active_connections);
        
        connections.fetch_add(1, Ordering::Relaxed);
        
        tokio::spawn(async move {
            handle_client(stream, cache_clone, pool_clone, connections.clone()).await;
            connections.fetch_sub(1, Ordering::Relaxed);
        });
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rustysquid=info".parse().unwrap()),
        )
        .init();

    info!("RustySquid v1.2.0 - HTTP Cache Proxy with Connection Pooling");
    info!("Listening on port {}", PROXY_PORT);
    info!("Cache size: {} entries", CACHE_SIZE);
    info!("Max connections: {}", MAX_CONNECTIONS);
    info!("Max cached response: {} MB", MAX_RESPONSE_SIZE / 1_048_576);

    // Initialize cache and connection pool
    let cache = ProxyCache::new();
    let pool = ConnectionPool::new();

    // Bind to port
    let listener = match TcpListener::bind(("0.0.0.0", PROXY_PORT)).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to port {}: {}", PROXY_PORT, e);
            std::process::exit(1);
        }
    };

    // Handle shutdown signals
    let shutdown = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C handler");
        info!("Shutting down gracefully...");
    };

    // Run server
    tokio::select! {
        _ = accept_connections(listener, cache, pool) => {},
        _ = shutdown => {},
    }
}
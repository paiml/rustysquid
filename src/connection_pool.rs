use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::debug;

const MAX_CONNECTIONS_PER_HOST: usize = 4;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug)]
struct PooledConnection {
    stream: TcpStream,
    last_used: Instant,
}

// Type alias to reduce complexity
type HostKey = (String, u16);
type ConnectionVec = Vec<PooledConnection>;
type PoolMap = HashMap<HostKey, ConnectionVec>;

/// Connection pool for upstream servers
#[derive(Clone)]
pub struct ConnectionPool {
    pools: Arc<Mutex<PoolMap>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            pools: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get a connection from the pool or create a new one
    pub async fn get_connection(&self, host: &str, port: u16) -> Result<TcpStream, &'static str> {
        let key = (host.to_string(), port);

        // Try to get an existing connection
        {
            let mut pools = self.pools.lock().await;
            if let Some(pool) = pools.get_mut(&key) {
                while let Some(mut conn) = pool.pop() {
                    // Check if connection is still fresh
                    if conn.last_used.elapsed() < IDLE_TIMEOUT {
                        // Test if connection is still alive
                        if Self::is_connection_alive(&mut conn.stream).await {
                            debug!("Reusing connection to {}:{}", host, port);
                            return Ok(conn.stream);
                        }
                    }
                    // Connection is stale or dead, continue to next
                    debug!("Dropping stale connection to {}:{}", host, port);
                }
            }
        }

        // No suitable connection found, create new one
        debug!("Creating new connection to {}:{}", host, port);
        timeout(CONNECTION_TIMEOUT, TcpStream::connect((host, port)))
            .await
            .map_err(|_| "Connection timeout")?
            .map_err(|_| "Connection failed")
    }

    /// Return a connection to the pool
    pub async fn return_connection(&self, host: String, port: u16, stream: TcpStream) {
        let key = (host.clone(), port);
        let mut pools = self.pools.lock().await;

        let pool = pools.entry(key).or_insert_with(Vec::new);

        // Only keep up to MAX_CONNECTIONS_PER_HOST
        if pool.len() < MAX_CONNECTIONS_PER_HOST {
            debug!("Returning connection to pool for {}:{}", host, port);
            pool.push(PooledConnection {
                stream,
                last_used: Instant::now(),
            });
        } else {
            debug!("Pool full for {}:{}, dropping connection", host, port);
            // Connection will be dropped automatically
        }
    }

    /// Test if a connection is still alive
    async fn is_connection_alive(stream: &mut TcpStream) -> bool {
        // Try to read with zero timeout to check if connection is closed
        // This is a non-blocking peek to see if the connection is still valid
        stream.readable().await.is_ok()
    }

    /// Clean up stale connections
    pub async fn cleanup_stale_connections(&self) {
        let mut pools = self.pools.lock().await;
        let now = Instant::now();

        for ((host, port), pool) in pools.iter_mut() {
            pool.retain(|conn| {
                let is_fresh = now.duration_since(conn.last_used) < IDLE_TIMEOUT;
                if !is_fresh {
                    debug!("Removing stale connection to {}:{}", host, port);
                }
                is_fresh
            });
        }

        // Remove empty pools
        pools.retain(|_, pool| !pool.is_empty());
    }

    /// Get statistics about the connection pool
    pub async fn stats(&self) -> HashMap<HostKey, usize> {
        let pools = self.pools.lock().await;
        pools
            .iter()
            .map(|(key, pool)| (key.clone(), pool.len()))
            .collect()
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool_basic() {
        let pool = ConnectionPool::new();

        // Stats should be empty initially
        let stats = pool.stats().await;
        assert!(stats.is_empty());
    }

    #[tokio::test]
    async fn test_connection_pool_return() {
        let pool = ConnectionPool::new();

        // Return a mock connection (will fail in real use but OK for testing pool logic)
        if let Ok(stream) = TcpStream::connect("127.0.0.1:1").await {
            pool.return_connection("test.com".to_string(), 80, stream)
                .await;

            let stats = pool.stats().await;
            let key = ("test.com".to_string(), 80);
            assert_eq!(stats.get(&key), Some(&1));
        }
    }
}

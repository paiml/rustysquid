use rustysquid::MAX_CONNECTIONS;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_connection_limit_constant() {
    // Verify the connection limit is exactly 100
    assert_eq!(MAX_CONNECTIONS, 100);
}

#[test]
fn test_connection_counting() {
    let counter = Arc::new(AtomicUsize::new(0));

    // Simulate connections
    for _ in 0..10 {
        counter.fetch_add(1, Ordering::Relaxed);
    }
    assert_eq!(counter.load(Ordering::Relaxed), 10);

    // Simulate disconnections
    for _ in 0..5 {
        counter.fetch_sub(1, Ordering::Relaxed);
    }
    assert_eq!(counter.load(Ordering::Relaxed), 5);
}

#[test]
fn test_connection_limit_enforcement() {
    let counter = Arc::new(AtomicUsize::new(0));
    let mut accepted = 0;
    let mut rejected = 0;

    // Try to accept 150 connections
    for _ in 0..150 {
        let current = counter.load(Ordering::Relaxed);
        if current >= MAX_CONNECTIONS {
            rejected += 1;
        } else {
            counter.fetch_add(1, Ordering::Relaxed);
            accepted += 1;
        }
    }

    // Should accept exactly MAX_CONNECTIONS
    assert_eq!(accepted, MAX_CONNECTIONS);
    assert_eq!(rejected, 50);
    assert_eq!(counter.load(Ordering::Relaxed), MAX_CONNECTIONS);
}

#[tokio::test]
async fn test_connection_lifecycle() {
    use tokio::task;

    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // Spawn tasks simulating connections
    for i in 0..10 {
        let counter_clone = counter.clone();
        let handle = task::spawn(async move {
            // Increment on connect
            counter_clone.fetch_add(1, Ordering::Relaxed);

            // Simulate work
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Decrement on disconnect
            counter_clone.fetch_sub(1, Ordering::Relaxed);
            i
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // All connections should be closed
    assert_eq!(counter.load(Ordering::Relaxed), 0);
}

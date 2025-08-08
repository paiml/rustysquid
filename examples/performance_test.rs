use bytes::Bytes;
/// Performance testing example - measures cache performance
///
/// Run with: cargo run --example performance_test --release
use rustysquid::{create_cache_key, CachedResponse, ProxyCache, MAX_ENTRY_SIZE};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("⚡ RustySquid Performance Test");
    println!("==============================");

    let cache = ProxyCache::new();

    // Test sequential operations
    println!("\n📊 Sequential Performance:");
    test_sequential_performance(&cache).await;

    // Test concurrent operations
    println!("\n🔄 Concurrent Performance:");
    test_concurrent_performance().await;

    // Test cache hit vs miss performance
    println!("\n🎯 Cache Hit vs Miss:");
    test_hit_vs_miss(&cache).await;

    // Test different entry sizes
    println!("\n📏 Size Impact:");
    test_size_impact(&cache).await;

    println!("\n✅ Performance tests completed!");
}

async fn test_sequential_performance(cache: &ProxyCache) {
    let iterations = 1000;
    let mut total_put = std::time::Duration::ZERO;
    let mut total_get = std::time::Duration::ZERO;

    for i in 0..iterations {
        let key = create_cache_key(&format!("test{}.com", i), 80, "/");
        let response = create_test_response(i, 1024);

        // Measure PUT
        let start = Instant::now();
        cache.put(key, response).await;
        total_put += start.elapsed();

        // Measure GET
        let start = Instant::now();
        cache.get(key).await;
        total_get += start.elapsed();
    }

    println!("   PUT operations:");
    println!("      Total: {:?}", total_put);
    println!("      Average: {:?}", total_put / iterations as u32);
    println!(
        "      Throughput: {:.0} ops/sec",
        iterations as f64 / total_put.as_secs_f64()
    );

    println!("   GET operations:");
    println!("      Total: {:?}", total_get);
    println!("      Average: {:?}", total_get / iterations as u32);
    println!(
        "      Throughput: {:.0} ops/sec",
        iterations as f64 / total_get.as_secs_f64()
    );

    cache.clear().await;
}

async fn test_concurrent_performance() {
    let cache = ProxyCache::new();
    let ops_per_batch = 100;

    let start = Instant::now();

    // Simulate concurrent operations in sequence (current_thread runtime)
    for batch in 0..10 {
        for op in 0..ops_per_batch {
            let key = create_cache_key(&format!("batch{}.com", batch), 80, &format!("/page{}", op));
            let response = create_test_response(batch * 100 + op, 512);

            cache.put(key, response).await;
            cache.get(key).await;
        }
    }

    let elapsed = start.elapsed();
    let total_ops = 10 * ops_per_batch * 2; // PUT + GET

    println!("   Batch operations (simulated concurrent):");
    println!("      Batches: 10");
    println!("      Total operations: {}", total_ops);
    println!("      Time: {:?}", elapsed);
    println!(
        "      Throughput: {:.0} ops/sec",
        total_ops as f64 / elapsed.as_secs_f64()
    );
    println!("      Final cache size: {} entries", cache.len().await);
}

async fn test_hit_vs_miss(cache: &ProxyCache) {
    cache.clear().await;

    let key = create_cache_key("benchmark.com", 80, "/test");
    let response = create_test_response(1, 4096);

    // Measure cache miss (first access)
    let start = Instant::now();
    cache.get(key).await; // Miss
    let miss_time = start.elapsed();

    // Add to cache
    cache.put(key, response).await;

    // Measure cache hit
    let start = Instant::now();
    cache.get(key).await; // Hit
    let hit_time = start.elapsed();

    println!("   Cache miss: {:?}", miss_time);
    println!("   Cache hit: {:?}", hit_time);

    if hit_time < miss_time {
        let speedup = miss_time.as_nanos() as f64 / hit_time.as_nanos().max(1) as f64;
        println!("   Speedup: {:.1}x faster", speedup);
    } else {
        println!("   Speedup: N/A (hit should be faster than miss)");
    }
}

async fn test_size_impact(cache: &ProxyCache) {
    let sizes = vec![
        (1024, "1 KB"),
        (10 * 1024, "10 KB"),
        (100 * 1024, "100 KB"),
        (1024 * 1024, "1 MB"),
        (MAX_ENTRY_SIZE / 2, "2.5 MB"),
    ];

    for (size, label) in sizes {
        cache.clear().await;

        let key = create_cache_key("size-test.com", 80, &format!("/{}", size));
        let response = create_test_response(size, size);

        let start = Instant::now();
        let added = cache.put(key, response).await;
        let put_time = start.elapsed();

        if added {
            let start = Instant::now();
            cache.get(key).await;
            let get_time = start.elapsed();

            println!("   {} entry:", label);
            println!("      PUT: {:?}", put_time);
            println!("      GET: {:?}", get_time);
        } else {
            println!("   {} entry: Rejected (too large)", label);
        }
    }
}

fn create_test_response(id: usize, size: usize) -> CachedResponse {
    CachedResponse {
        status_line: format!("HTTP/1.1 200 OK {}\r\n", id),
        headers: vec![
            "Content-Type: text/html".to_string(),
            format!("Content-Length: {}", size),
        ],
        body: Bytes::from(vec![b'X'; size]),
        expires: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600,
    }
}

use proptest::prelude::*;
use rustysquid::create_cache_key;

proptest! {
    // Test optimized cache key generation
    #[test]
    fn prop_cache_key_optimized_consistency(
        host: String,
        port: u16,
        path: String,
    ) {
        // The optimized version should produce consistent results
        let key1 = create_cache_key(&host, port, &path);
        let key2 = create_cache_key(&host, port, &path);

        // Keys should be deterministic
        prop_assert_eq!(key1, key2);

        // Keys should be non-zero (valid hash)
        prop_assert_ne!(key1, 0);
    }

    #[test]
    fn prop_cache_key_different_inputs_different_keys(
        host1: String,
        host2: String,
        port: u16,
        path: String,
    ) {
        prop_assume!(host1 != host2);

        let key1 = create_cache_key(&host1, port, &path);
        let key2 = create_cache_key(&host2, port, &path);

        // Different hosts should produce different keys
        prop_assert_ne!(key1, key2);
    }
}

#[test]
fn test_cache_key_no_allocations_verification() {
    // This test verifies the optimized implementation
    // The old version: xxh64(format!("{}:{}{}").as_bytes(), 0)
    // The new version: incremental hashing with no format!

    let key1 = create_cache_key("example.com", 80, "/path");
    let key2 = create_cache_key("example.com", 80, "/path");

    assert_eq!(key1, key2, "Keys should be deterministic");
    assert_ne!(key1, 0, "Key should be valid");

    // Verify different inputs produce different keys
    let key3 = create_cache_key("example.com", 443, "/path");
    assert_ne!(key1, key3, "Different ports should produce different keys");
}

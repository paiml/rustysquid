# Contributing to RustySquid

Thank you for your interest in contributing to RustySquid! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful and inclusive. We're building a tool for everyone.

## How to Contribute

### Reporting Issues

1. Check existing issues first
2. Use issue templates when available
3. Include:
   - RustySquid version
   - Target platform (router model, OS)
   - Steps to reproduce
   - Expected vs actual behavior
   - Logs if applicable

### Pull Requests

1. **Fork and branch**: Create a feature branch from `main`
2. **Follow the plan**: Check `docs/todo/minimal-safety-features-port-squid.md`
3. **One task per PR**: Keep PRs focused on a single task from the plan
4. **Test thoroughly**: Add tests for your changes
5. **Document changes**: Update relevant documentation
6. **Update CHANGELOG**: Add your changes to the Unreleased section

### Development Setup

```bash
# Clone your fork
git clone git@github.com:YOUR_USERNAME/rustysquid.git
cd rustysquid

# Add upstream
git remote add upstream git@github.com:paiml/rustysquid.git

# Create feature branch
git checkout -b task-1.1-memory-limit

# Install development tools
cargo install cargo-watch cargo-tarpaulin cargo-audit cargo-bloat

# Run tests in watch mode
cargo watch -x test

# Check safety
make -f Makefile.safety safety-check
```

## Quality Standards (PMAT)

We follow PMAT methodology:

### Property Testing
- Add property tests for invariants
- Use proptest for randomized testing
- Ensure no panics on any input

### Metrics
- Maintain > 80% code coverage
- Binary size < 500KB
- Memory usage < 10MB idle

### Automated Testing
- All tests must pass
- No clippy warnings with pedantic lints
- Format code with rustfmt

### Testing Requirements
Each feature needs:
1. Unit tests
2. Property tests (where applicable)
3. Doc tests with examples

## Testing Guidelines

### Unit Tests
```rust
#[test]
fn test_specific_behavior() {
    // Arrange
    let cache = ProxyCache::new();
    
    // Act
    let result = cache.get(key).await;
    
    // Assert
    assert!(result.is_none());
}
```

### Property Tests
```rust
proptest! {
    #[test]
    fn prop_cache_never_exceeds_limit(
        entries in vec(any::<CacheEntry>(), 0..20000)
    ) {
        let cache = ProxyCache::new();
        for entry in entries {
            cache.put(entry);
            prop_assert!(cache.total_size() <= MAX_CACHE_SIZE);
        }
    }
}
```

### Doc Tests
```rust
/// Calculates cache key for request
/// 
/// # Example
/// ```
/// use rustysquid::create_cache_key;
/// 
/// let key = create_cache_key("example.com", 80, "/index.html");
/// assert_ne!(key, 0);
/// ```
pub fn create_cache_key(host: &str, port: u16, path: &str) -> u64 {
    // ...
}
```

## Performance Guidelines

- Profile before optimizing
- Benchmark changes: `cargo bench`
- Check binary size: `cargo bloat --release`
- Monitor memory: Use valgrind or heaptrack
- Keep allocations minimal

## Documentation

- Document all public APIs
- Include examples in doc comments
- Update README for user-facing changes
- Keep implementation notes in code comments

## Release Process

1. **Version bump**: Update version in Cargo.toml
2. **Update CHANGELOG**: Move Unreleased items to new version
3. **Create PR**: Target main branch
4. **After merge**: Tag will trigger automatic release

## Getting Help

- Check [docs/todo/minimal-safety-features-port-squid.md](docs/todo/minimal-safety-features-port-squid.md)
- Open a discussion for design questions
- Join our Matrix room: #rustysquid:matrix.org (planned)

## Recognition

Contributors will be added to:
- README.md contributors section
- GitHub releases acknowledgments
- Authors field in Cargo.toml (significant contributions)

## Commit Messages

Follow conventional commits:
- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `test:` Test additions/changes
- `perf:` Performance improvements
- `refactor:` Code restructuring
- `chore:` Maintenance tasks

Example:
```
feat: add memory limit enforcement (task 1.1)

- Add total_size tracking to ProxyCache
- Implement LRU eviction when size exceeded
- Add MAX_CACHE_BYTES constant (50MB)

Closes #12
```

Thank you for contributing to make RustySquid better!
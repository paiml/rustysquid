# Release Checklist for RustySquid 0.1.0

## Pre-Release Requirements

### Safety (Must Have)
- [ ] All Phase 1 tasks complete (Memory Safety)
- [ ] All Phase 2 tasks complete (Request Safety)  
- [ ] All Phase 3 tasks complete (Error Handling)
- [ ] Zero `unwrap()` calls in production code
- [ ] Zero `println!` calls (replaced with tracing)
- [ ] All `let _ =` patterns addressed

### Quality Metrics
- [ ] Code coverage > 80%
- [ ] Binary size < 500KB (stripped)
- [ ] Memory usage < 10MB (idle)
- [ ] All tests passing (unit, property, doc)
- [ ] No clippy warnings with pedantic lints
- [ ] cargo audit shows no vulnerabilities

### Documentation
- [ ] README.md updated with current features
- [ ] CHANGELOG.md updated for release
- [ ] API documentation complete (`cargo doc`)
- [ ] Examples directory with usage samples
- [ ] Installation guide for routers

### Testing
- [ ] 24-hour fuzz testing with no panics
- [ ] Tested on actual router hardware
- [ ] Performance benchmarks documented
- [ ] Memory leak testing with valgrind
- [ ] Cross-platform builds verified

## Release Process

### 1. Version Preparation
```bash
# Update version in Cargo.toml
cargo set-version 0.1.0-rc.1

# Update CHANGELOG.md
# Move items from Unreleased to new version section

# Commit changes
git add Cargo.toml CHANGELOG.md
git commit -m "chore: prepare v0.1.0-rc.1 release"
```

### 2. Testing Release Candidate
```bash
# Create release candidate tag
git tag v0.1.0-rc.1
git push origin v0.1.0-rc.1

# This triggers GitHub Actions to:
# - Build binaries for all platforms
# - Run full test suite
# - Create draft release
```

### 3. Beta Testing (1-2 weeks)
- [ ] Deploy to test routers
- [ ] Monitor for crashes
- [ ] Collect performance metrics
- [ ] Get user feedback
- [ ] Fix any critical issues

### 4. Final Release
```bash
# Update to final version
cargo set-version 0.1.0
git add Cargo.toml
git commit -m "chore: release v0.1.0"

# Create release tag
git tag v0.1.0
git push origin v0.1.0

# Publish to crates.io
cargo publish
```

### 5. Post-Release
- [ ] Verify crates.io publication
- [ ] Verify GitHub release with binaries
- [ ] Update installation documentation
- [ ] Announce on:
  - [ ] Reddit r/rust
  - [ ] Reddit r/selfhosted  
  - [ ] Router forums
  - [ ] Twitter/Mastodon
- [ ] Create Docker image
- [ ] Submit to package managers

## Platform Binaries

Ensure all binaries are built and tested:
- [ ] x86_64-unknown-linux-musl
- [ ] aarch64-unknown-linux-musl (routers)
- [ ] armv7-unknown-linux-musleabihf
- [ ] x86_64-apple-darwin
- [ ] aarch64-apple-darwin
- [ ] x86_64-pc-windows-msvc

## Quality Gates

Before releasing, verify:
```bash
# No unsafe code
grep -r "unsafe" src/ | wc -l  # Should be 0

# No panicking code
grep -r "unwrap()" src/ | wc -l  # Should be 0
grep -r "expect(" src/ | wc -l   # Should have context
grep -r "panic!" src/ | wc -l    # Should be 0

# Size check
cargo build --release --target aarch64-unknown-linux-musl
ls -lh target/aarch64-unknown-linux-musl/release/rustysquid
# Should be < 500KB

# Security check
cargo audit

# Test coverage
cargo tarpaulin --out Html
# Should be > 80%
```

## Rollback Plan

If critical issues found post-release:
1. `cargo yank --version 0.1.0` (removes from crates.io)
2. Delete GitHub release (keeps tag)
3. Fix issues
4. Release as 0.1.1 with fixes

## Success Metrics

Target for first week after release:
- [ ] 100+ downloads on crates.io
- [ ] 10+ GitHub stars
- [ ] 0 critical bug reports
- [ ] 5+ successful router deployments
- [ ] Performance meets targets:
  - Cache hit latency < 1ms
  - Memory usage < 50MB under load
  - 99.9% uptime

## Notes

- Always release on Tuesday-Thursday (avoid Fridays)
- Have rollback plan ready
- Monitor GitHub issues closely for first 48 hours
- Be ready to release 0.1.1 quickly if needed
# Contributing to LLM Archive V2

Thank you for your interest in contributing to LLM Archive V2! This document provides guidelines and instructions for contributing.

## Development Setup

### Prerequisites
- Rust 1.74 or later
- Python 3.8+ (for PyO3 bridge during migration)
- SQLite 3.35+ (for FTS5 support)

### Building from Source
```bash
# Clone the repository
git clone https://github.com/SahandTava/llm-archive-v2.git
cd llm-archive-v2

# Build in debug mode
cargo build

# Build optimized release
cargo build --release

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- serve
```

## Code Style

### Rust Guidelines
- Follow standard Rust naming conventions
- Use `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Keep functions focused and under 50 lines
- Document public APIs with doc comments

### Examples
```rust
/// Parse a conversation from export data
/// 
/// # Arguments
/// * `data` - Raw JSON export data
/// 
/// # Returns
/// * `Result<(Conversation, Vec<Message>)>` - Parsed conversation and messages
pub fn parse_conversation(data: &Value) -> Result<(Conversation, Vec<Message>)> {
    // Implementation
}
```

## Adding New Features

### 1. New Parser Support
To add support for a new LLM provider:

1. Create parser module in `src/import/parsers/`
2. Define export format structures
3. Implement parsing logic
4. Add to provider enum in `src/models.rs`
5. Update import module to use new parser
6. Add Python wrapper in `parsers/` directory
7. Write tests with sample data

### 2. Search Improvements
- Keep FTS5 queries optimized
- Test with large datasets (1M+ messages)
- Profile before adding complexity
- Consider memory usage

### 3. UI Enhancements
- Maintain keyboard navigation
- Keep pages under 100KB
- Support dark mode
- Test on various screen sizes

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_chatgpt_export() {
        let data = include_str!("../test_data/chatgpt_sample.json");
        let result = parse_export(data);
        assert!(result.is_ok());
    }
}
```

### Integration Tests
Place in `tests/` directory:
```rust
#[tokio::test]
async fn test_import_and_search() {
    // Test full import -> search flow
}
```

### Performance Tests
```bash
# Benchmark with criterion
cargo bench

# Profile with flamegraph
cargo flamegraph --bin llm-archive -- import chatgpt large_export.json
```

## Submitting Changes

### Pull Request Process
1. Fork the repository
2. Create feature branch: `git checkout -b feature/amazing-feature`
3. Make changes and test thoroughly
4. Commit with clear message: `git commit -m 'Add amazing feature'`
5. Push to branch: `git push origin feature/amazing-feature`
6. Open Pull Request with description

### PR Requirements
- [ ] Tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Documentation updated
- [ ] CHANGELOG.md entry added
- [ ] Performance impact considered

### Commit Messages
Follow conventional commits:
- `feat:` New feature
- `fix:` Bug fix
- `perf:` Performance improvement
- `docs:` Documentation only
- `refactor:` Code refactoring
- `test:` Test additions/changes

## Performance Considerations

### Critical Paths
These areas are performance-critical:
1. Message parsing (called millions of times)
2. FTS5 search queries
3. Database batch inserts
4. HTTP request handling

### Benchmarking
Before optimizing:
1. Profile with real data
2. Measure baseline performance
3. Make focused changes
4. Verify improvements
5. Document in PR

## Architecture Principles

### Keep It Simple
- No unnecessary abstractions
- Direct, readable code
- Flat structures over deep hierarchies
- Explicit over implicit

### Performance First
- Profile before optimizing
- Batch operations when possible
- Use streaming for large data
- Minimize allocations

### User Experience
- Sub-second response times
- Clear error messages
- Keyboard navigation
- Progressive enhancement

## Getting Help

- GitHub Issues: Bug reports and feature requests
- Discussions: General questions and ideas
- Email: [contact info if applicable]

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
# LLM Archive V2

A fast, focused tool for importing and searching LLM conversation archives.

## 🚀 Key Features

- **Sub-second search** across millions of messages using SQLite FTS5
- **50ms cold start** with single binary deployment
- **Import support** for ChatGPT, Claude, Gemini, XAI, and more
- **Desktop-first UI** with keyboard navigation
- **< 20MB memory usage** even with large archives

## 📊 Performance Targets

| Metric | Target | V1 Baseline |
|--------|--------|-------------|
| Cold Start | < 50ms | 3-5 seconds |
| Search Response | < 100ms | Broken |
| Memory Usage | < 20MB | 300MB+ |
| Import Speed | 10k msgs/sec | ~100 msgs/sec |
| Binary Size | < 15MB | N/A (Python) |

## 🏗️ Architecture

```
Single Rust Binary
    ├── Web Server (Axum)
    ├── SQLite + FTS5
    ├── PyO3 Bridge (temporary)
    └── Embedded Assets
```

## 🚦 Quick Start

```bash
# Install and run
cargo build --release
./target/release/llm-archive

# Import conversations
llm-archive import chatgpt ./exports/conversations.json
llm-archive import claude ./exports/claude-*.json

# Start web server
llm-archive serve --port 8080
```

## 📁 Project Structure

```
src/
├── main.rs              # Entry point
├── server/              # Web server (Axum)
├── models/              # Domain models
├── db/                  # Database layer
├── import/              # Import pipelines
│   ├── mod.rs          # Import orchestration
│   ├── python_bridge.rs # PyO3 integration
│   └── parsers/        # Native Rust parsers
├── search/              # FTS5 search
└── templates/           # Askama templates
```

## 🔄 Migration Status

- [x] Core architecture setup
- [x] Database schema (simplified to 4 tables)
- [ ] PyO3 bridge for Python parsers
- [ ] Native ChatGPT parser
- [ ] Native Claude parser
- [ ] Search implementation
- [ ] Web UI
- [ ] Keyboard navigation

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with golden test data
cargo test --features golden-tests

# Benchmark performance
cargo bench
```

## 📈 Observability

Built-in metrics exposed at `/metrics`:
- Request latency histograms
- Search performance metrics
- Import throughput counters
- Memory usage gauges

## 🔧 Configuration

```toml
# config.toml
[database]
path = "./llm_archive.db"
wal_mode = true
mmap_size = 1073741824  # 1GB

[search]
max_results = 100
snippet_length = 200

[import]
batch_size = 1000
python_bridge = true  # Use Python parsers initially
```

## 🤝 Contributing

1. Keep it simple - no unnecessary abstractions
2. Performance first - profile before optimizing
3. Test with real data - use golden test files
4. Document decisions - update ARCHITECTURE.md

## 📝 License

MIT License - see LICENSE file for details
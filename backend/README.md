# LLM Archive V2 Backend

High-performance Rust backend optimized for <100ms response times.

## Quick Start

```bash
# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Run in development
DATABASE_URL=sqlite:llm_archive.db cargo run

# Build for production
cargo build --release
```

## API Endpoints

- `GET /api/search?q={query}` - Search messages
- `GET /api/conversations?limit=50&offset=0` - List conversations
- `GET /api/conversations/{id}` - Get conversation details
- `GET /api/conversations/{id}/messages` - Get messages
- `POST /api/import` - Import chat data

## Import Format

```json
{
  "provider": "chatgpt",
  "data": { /* ChatGPT JSON export */ }
}
```

Server runs on http://localhost:3000
# LLM Archive V2 Deployment Guide

## Quick Start

```bash
# Clone and build
git clone <your-repo-url>
cd llm-archive-v2
cargo build --release

# Initialize database
./target/release/llm-archive init

# Import data
./target/release/llm-archive import chatgpt /path/to/conversations.json

# Start server
./target/release/llm-archive serve --port 8080
```

## Production Deployment

### 1. Single Binary Deployment

The entire application compiles to a single static binary:

```bash
# Build optimized binary
cargo build --release

# Copy to server (only file needed!)
scp target/release/llm-archive user@server:/usr/local/bin/

# Run on server
ssh user@server
llm-archive serve --database /var/lib/llm-archive/data.db
```

### 2. Systemd Service

Create `/etc/systemd/system/llm-archive.service`:

```ini
[Unit]
Description=LLM Archive V2
After=network.target

[Service]
Type=simple
User=llm-archive
Group=llm-archive
WorkingDirectory=/var/lib/llm-archive
ExecStart=/usr/local/bin/llm-archive serve --port 8080 --database /var/lib/llm-archive/data.db
Restart=on-failure
RestartSec=5

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/llm-archive

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable llm-archive
sudo systemctl start llm-archive
```

### 3. Reverse Proxy (Nginx)

```nginx
server {
    listen 80;
    server_name llm-archive.example.com;
    
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket support (if needed later)
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

### 4. Docker Deployment

```dockerfile
# Dockerfile
FROM rust:1.74 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/llm-archive /usr/local/bin/
EXPOSE 8080
CMD ["llm-archive", "serve"]
```

Build and run:
```bash
docker build -t llm-archive .
docker run -p 8080:8080 -v ./data:/data llm-archive
```

### 5. Environment Variables

```bash
# Database location
export LLM_ARCHIVE_DB=/var/lib/llm-archive/data.db

# Server configuration
export LLM_ARCHIVE_HOST=0.0.0.0
export LLM_ARCHIVE_PORT=8080

# Logging
export RUST_LOG=info,llm_archive=debug
```

## Monitoring

### Health Check
```bash
curl http://localhost:8080/health
```

### Metrics (Prometheus format)
```bash
curl http://localhost:8080/metrics
```

### Logs
```bash
# With systemd
journalctl -u llm-archive -f

# With Docker
docker logs -f llm-archive
```

## Backup & Restore

### Backup
```bash
# SQLite backup (safe during operation)
sqlite3 /var/lib/llm-archive/data.db ".backup /backup/llm-archive-$(date +%Y%m%d).db"

# Or use the built-in backup command (coming soon)
llm-archive backup /backup/llm-archive-$(date +%Y%m%d).db
```

### Restore
```bash
# Stop service
sudo systemctl stop llm-archive

# Replace database
cp /backup/llm-archive-20240115.db /var/lib/llm-archive/data.db

# Start service
sudo systemctl start llm-archive
```

## Performance Tuning

### 1. Database Optimization
```bash
# Run periodically to optimize database
sqlite3 /var/lib/llm-archive/data.db "VACUUM; ANALYZE;"
```

### 2. System Limits
```bash
# /etc/security/limits.conf
llm-archive soft nofile 65536
llm-archive hard nofile 65536
```

### 3. Kernel Parameters
```bash
# /etc/sysctl.conf
net.core.somaxconn = 1024
net.ipv4.tcp_tw_reuse = 1
```

## Troubleshooting

### Database Locked
```bash
# Check for WAL files
ls -la /var/lib/llm-archive/data.db*

# If corrupted, recover
sqlite3 /var/lib/llm-archive/data.db "PRAGMA integrity_check;"
```

### High Memory Usage
```bash
# Check SQLite cache size
sqlite3 /var/lib/llm-archive/data.db "PRAGMA cache_size;"

# Reduce if needed in config.toml
[database]
cache_size = -32000  # 32MB instead of 64MB
```

### Slow Searches
```bash
# Rebuild FTS index
sqlite3 /var/lib/llm-archive/data.db "INSERT INTO messages_fts(messages_fts) VALUES('rebuild');"

# Check query plan
sqlite3 /var/lib/llm-archive/data.db "EXPLAIN QUERY PLAN SELECT ..."
```

## Security Checklist

- [ ] Run as non-root user
- [ ] Restrict file permissions: `chmod 600 /var/lib/llm-archive/data.db`
- [ ] Enable HTTPS with Let's Encrypt
- [ ] Set up fail2ban for brute force protection
- [ ] Regular backups to separate location
- [ ] Monitor disk space for database growth
- [ ] Review logs for suspicious activity
# Build stage
FROM rust:1.82-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY apps/ apps/

# Build release binary
RUN cargo build --release -p arbitrage-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/arbitrage-bot /app/arbitrage-bot

# Copy configuration files (if any)
COPY symbol_mappings.json* ./
COPY network_name_mapping.json* ./

# Create data directory for SQLite
RUN mkdir -p /app/data

# Environment variables
ENV RUST_LOG=info
ENV TELEGRAM_BOT_TOKEN=""
ENV DATABASE_URL=sqlite:/app/data/alerts.db

# Expose WebSocket port
EXPOSE 9001

# Run the bot with live feeds and Telegram alerts
ENTRYPOINT ["/app/arbitrage-bot"]
CMD ["--live", "--telegram", "--db-path", "/app/data/alerts.db"]

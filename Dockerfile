# Multi-stage Dockerfile for Health Export REST API
# Optimized for production with minimal container size (<100MB target)

# Build stage
FROM rust:1.75-alpine AS builder

# Install system dependencies required for building
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static \
    postgresql-dev

# Create app user and group for security
RUN addgroup -g 1001 -S appgroup && \
    adduser -S -D -H -u 1001 -g appgroup appuser

# Set working directory
WORKDIR /app

# Copy dependency files first for better Docker layer caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main to compile dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs

# Build dependencies (this layer will be cached unless Cargo files change)
RUN cargo build --release && \
    rm -rf src/

# Copy source code
COPY src/ ./src/
COPY migrations/ ./migrations/

# Build the application
# Use static linking for minimal runtime dependencies
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN cargo build --release --target x86_64-unknown-linux-musl

# Strip the binary to reduce size
RUN strip target/x86_64-unknown-linux-musl/release/self-sensored

# Runtime stage - minimal Alpine image
FROM alpine:3.18 AS runtime

# Install only essential runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    tzdata \
    curl

# Create app user and group (same IDs as builder stage)
RUN addgroup -g 1001 -S appgroup && \
    adduser -S -D -H -u 1001 -g appgroup appuser

# Create app directory with proper ownership
RUN mkdir -p /app && \
    chown -R appuser:appgroup /app

# Set working directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder --chown=appuser:appgroup /app/target/x86_64-unknown-linux-musl/release/self-sensored ./

# Copy migrations directory (needed for database operations)
COPY --from=builder --chown=appuser:appgroup /app/migrations ./migrations/

# Create config directory for runtime configuration
RUN mkdir -p config && \
    chown appuser:appgroup config

# Switch to non-root user for security
USER appuser

# Expose application port
EXPOSE 8080

# Expose metrics port
EXPOSE 9090

# Health check using the optimized liveness probe endpoint
HEALTHCHECK --interval=15s --timeout=5s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8080/health/live || exit 1

# Environment variables with secure defaults
ENV RUST_LOG=info
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV METRICS_PORT=9090
ENV ENVIRONMENT=production

# Set the binary as entrypoint
ENTRYPOINT ["./self-sensored"]

# Labels for better container management
LABEL maintainer="Health Export Team <team@example.com>"
LABEL version="1.0.0"
LABEL description="Health Export REST API - Optimized production container"
LABEL org.opencontainers.image.source="https://github.com/health-export/api"
LABEL org.opencontainers.image.title="Health Export API"
LABEL org.opencontainers.image.description="Rust-based REST API for health data ingestion"
LABEL org.opencontainers.image.vendor="Health Export Team"
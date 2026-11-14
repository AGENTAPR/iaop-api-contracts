# Multi-stage Dockerfile for IAOP Central Developer Portal
# Stage 1: Build binary
FROM rust:slim-bookworm AS builder

WORKDIR /usr/src/iaop-api-contracts

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy workspace and crate manifests
COPY Cargo.toml Cargo.lock ./
COPY portal/Cargo.toml ./portal/

# Cache dependency layer
RUN mkdir -p portal/src && echo "fn main() {}" > portal/src/main.rs && echo "" > portal/src/lib.rs
RUN cargo build --release -p iaop-developer-portal || true

# Copy full source and catalog/specs
COPY portal ./portal
COPY catalog ./catalog
COPY specs ./specs

# Build release binary
RUN touch portal/src/main.rs portal/src/lib.rs && cargo build --release -p iaop-developer-portal

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install ca-certificates and curl for health probes
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*

# Create non-root system user
RUN groupadd -g 10001 appuser && useradd -u 10001 -g appuser -s /bin/sh appuser

# Copy compiled binary from builder
COPY --from=builder /usr/src/iaop-api-contracts/target/release/iaop-developer-portal /app/iaop-developer-portal

# Copy static specifications and catalog definitions
COPY --from=builder --chown=appuser:appuser /usr/src/iaop-api-contracts/catalog /app/catalog
COPY --from=builder --chown=appuser:appuser /usr/src/iaop-api-contracts/specs /app/specs

# Set environment variables
ENV HOST=0.0.0.0 \
    PORT=8080 \
    CATALOG_PATH=/app/catalog/services.yaml \
    CATALOG_SCHEMA_PATH=/app/catalog/catalog.schema.json \
    SPECS_DIR=/app/specs \
    ENVIRONMENT=development \
    RUST_LOG=info,iaop_developer_portal=info

EXPOSE 8080

USER appuser

# Healthcheck probe
HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health/live || exit 1

ENTRYPOINT ["/app/iaop-developer-portal"]

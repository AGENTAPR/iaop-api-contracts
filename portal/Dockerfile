# syntax=docker/dockerfile:1
# Multi-stage Dockerfile for IAOP Central Developer Portal V2 (Scalar)
#
# Stage 1: Build Rust binary + download Scalar JS bundle at build time.
#   - Scalar is pinned to 1.25.72 and downloaded once during image build.
#   - No CDN URLs are served in HTML at runtime; all assets are same-origin.
#
# Stage 2: Minimal Debian runtime image.

# ----- Stage 1: Builder -----
FROM rust:slim-bookworm AS builder

WORKDIR /usr/src/iaop-api-contracts

# Install build dependencies + curl for Scalar bundle download
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libssl-dev curl ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Download Scalar JS bundle at build time (pinned version — no CDN at runtime)
RUN curl -fsSL \
    "https://cdn.jsdelivr.net/npm/@scalar/api-reference@1.25.72/dist/browser/standalone.min.js" \
    -o /tmp/scalar.min.js \
 && echo "Scalar bundle downloaded ($(wc -c < /tmp/scalar.min.js) bytes)"

# Copy workspace and crate manifests for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY portal/Cargo.toml ./portal/

# Cache dependency layer (build dummy main/lib, then overwrite)
RUN mkdir -p portal/src \
 && echo "fn main() {}" > portal/src/main.rs \
 && echo "" > portal/src/lib.rs
RUN cargo build --release -p iaop-developer-portal || true

# Copy full source, catalog, and specs
COPY portal ./portal
COPY catalog ./catalog
COPY specs ./specs

# Build release binary
RUN touch portal/src/main.rs portal/src/lib.rs \
 && cargo build --release -p iaop-developer-portal

# ----- Stage 2: Runtime -----
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Install runtime deps (ca-certificates for TLS, curl for health probe)
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -g 10001 appuser \
 && useradd -u 10001 -g appuser -s /bin/sh appuser

# Copy compiled binary
COPY --from=builder /usr/src/iaop-api-contracts/target/release/iaop-developer-portal /app/iaop-developer-portal

# Copy catalog and spec files
COPY --from=builder --chown=appuser:appuser /usr/src/iaop-api-contracts/catalog /app/catalog
COPY --from=builder --chown=appuser:appuser /usr/src/iaop-api-contracts/specs /app/specs

# Bundle Scalar JS (pinned at build time — served from /static/, no CDN at runtime)
RUN mkdir -p /app/static
COPY --from=builder --chown=appuser:appuser /tmp/scalar.min.js /app/static/scalar.min.js

# Environment defaults
ENV HOST=0.0.0.0 \
    PORT=8080 \
    CATALOG_PATH=/app/catalog/services.yaml \
    CATALOG_SCHEMA_PATH=/app/catalog/catalog.schema.json \
    SPECS_DIR=/app/specs \
    SCALAR_JS_PATH=/app/static/scalar.min.js \
    ENVIRONMENT=development \
    ALLOWED_CORS_ORIGINS="" \
    RUST_LOG=info,iaop_developer_portal=info

EXPOSE 8080

USER appuser

HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health/live || exit 1

ENTRYPOINT ["/app/iaop-developer-portal"]

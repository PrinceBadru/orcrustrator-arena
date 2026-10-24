# ==============================================================================
# Multi-Stage Production Dockerfile: Orcrustrator Arena
# Optimized for Cloud Deployment (Render, Fly.io, Hugging Face Spaces)
# Low-memory footprint with capped compilation parallelism (<512MB RAM build limit)
# ==============================================================================

# Stage 1: Build the React + TypeScript Mission Control Dashboard
FROM node:22-alpine AS ui-builder
WORKDIR /app/ui
COPY ui/package*.json ./
RUN npm ci || npm install
COPY ui/ ./
RUN npm run build

# Stage 2: Build Rust Microservices, Gateways, Auditor, and Recorder
FROM rust:1.84-bookworm AS rust-builder
WORKDIR /app

# Cap parallelism and memory churn to ensure builds succeed on free-tier 512MB runners
ENV CARGO_BUILD_JOBS=2
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# Copy workspace manifest and source trees
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY services ./services
COPY gateways ./gateways
COPY auditor ./auditor
COPY recorder ./recorder

# Build all release binaries with bounded concurrency
RUN cargo build --release --workspace --jobs 2

# Stage 3: Minimal Production Debian Runtime (Matching Glibc ABI)
FROM debian:bookworm-slim AS runner
WORKDIR /app

# Install Nginx, envsubst, and essential runtime utilities
RUN apt-get update && apt-get install -y --no-install-recommends \
    nginx \
    gettext-base \
    ca-certificates \
    curl \
    procps \
    && rm -rf /var/lib/apt/lists/*

# Copy compiled Rust binaries into /app/bin
RUN mkdir -p /app/bin
COPY --from=rust-builder /app/target/release/registry /app/bin/
COPY --from=rust-builder /app/target/release/lab /app/bin/
COPY --from=rust-builder /app/target/release/pharmacy /app/bin/
COPY --from=rust-builder /app/target/release/terminology /app/bin/
COPY --from=rust-builder /app/target/release/billing /app/bin/
COPY --from=rust-builder /app/target/release/notifications /app/bin/
COPY --from=rust-builder /app/target/release/naive /app/bin/
COPY --from=rust-builder /app/target/release/hand-rolled /app/bin/
COPY --from=rust-builder /app/target/release/orcrustrator-gw /app/bin/
COPY --from=rust-builder /app/target/release/auditor /app/bin/
COPY --from=rust-builder /app/target/release/recorder /app/bin/

# Copy UI static distribution to Nginx web root
RUN mkdir -p /var/www/html
COPY --from=ui-builder /app/ui/dist /var/www/html

# Copy Nginx template and entrypoint
COPY nginx.conf.template /etc/nginx/conf.d/default.conf.template
COPY docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh /app/bin/*

ENV PORT=10000
EXPOSE 10000

ENTRYPOINT ["/docker-entrypoint.sh"]

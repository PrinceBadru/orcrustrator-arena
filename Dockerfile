# ==============================================================================
# Multi-Stage Production Dockerfile: Orcrustrator Arena
# Deploys UI + All Microservices + Gateways + Auditor in a single low-memory container (<80MB RAM)
# Compatible with 100% Free Tiers: Koyeb, Render, Fly.io, Hugging Face Spaces
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

# Copy workspace manifest and source trees
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY services ./services
COPY gateways ./gateways
COPY auditor ./auditor
COPY recorder ./recorder

# Build all release binaries in parallel
RUN cargo build --release --workspace

# Stage 3: Minimal Production Runtime
FROM nginx:alpine AS runner
WORKDIR /app

# Install runtime dependencies for dynamic $PORT substitution and glibc compatibility
RUN apk add --no-cache gettext ca-certificates libc6-compat libgcc

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
COPY --from=ui-builder /app/ui/dist /usr/share/nginx/html

# Copy Nginx template and entrypoint
COPY nginx.conf.template /etc/nginx/conf.d/default.conf.template
COPY docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh /app/bin/*

ENV PORT=3000
EXPOSE 3000

ENTRYPOINT ["/docker-entrypoint.sh"]

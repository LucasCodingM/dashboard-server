# --- Stage 1: Build ---
FROM rust:1.88-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY templates/ templates/

RUN cargo build --release --quiet

# --- Stage 2: Runtime ---
FROM debian:bookworm-slim AS runtime

# Docker CLI + plugin compose (pour contrôler le daemon host via socket monté)
COPY --from=docker:27-cli /usr/local/bin/docker /usr/local/bin/docker
COPY --from=docker:27-cli /usr/local/libexec/docker/cli-plugins/docker-compose /usr/local/libexec/docker/cli-plugins/docker-compose

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    wget \
 && rm -rf /var/lib/apt/lists/*

# yt-dlp
RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux \
      -o /usr/local/bin/yt-dlp \
 && chmod a+rx /usr/local/bin/yt-dlp

WORKDIR /app

COPY --from=builder /app/target/release/dashboard-server /usr/local/bin/dashboard-server
COPY static/ static/

# Groupe docker pour accès au socket — le GID est résolu à l'exécution via docker-compose
RUN useradd -m -u 10001 appuser
USER appuser

EXPOSE 3000

CMD ["/usr/local/bin/dashboard-server"]

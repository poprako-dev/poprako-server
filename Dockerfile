FROM rust:1.95.0-alpine3.22 AS builder

WORKDIR /work

RUN apk add --no-cache \
    build-base \
    cmake \
    curl \
    libpq-dev \
    perl \
    pkgconf

# Pre-build dependencies with a stub binary. The resulting target directory
# stays in the Docker layer, so source-only changes reuse compiled dependencies
# without risking stale artifacts from a shared target cache mount.
COPY Cargo.toml Cargo.lock ./
COPY poprako-util ./poprako-util
COPY poprako-swagger ./poprako-swagger
COPY benches ./benches

RUN mkdir -p src && \
    echo 'fn main() {}' > src/main.rs && \
    echo '' > src/lib.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    CARGO_INCREMENTAL=1 \
    cargo build --release --bin poprako-server && \
    rm -rf src

# Rebuild with actual source, reusing the dependency artifacts from the layer
# above.
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    CARGO_INCREMENTAL=1 \
    cargo clean --package poprako-server && \
    cargo build --release --bin poprako-server && \
    cp /work/target/release/poprako-server /work/poprako-server

FROM alpine:3.22 AS runtime

WORKDIR /app

RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    libpq && \
    addgroup -S poprako && \
    adduser -S -G poprako -h /app poprako && \
    touch /app/.env && \
    chown poprako:poprako /app/.env

COPY --from=builder --chown=poprako:poprako \
    /work/poprako-server /app/poprako-server
COPY --chown=poprako:poprako \
    deploy/poprako-server/application_config.json /app/application_config.json

USER poprako

EXPOSE 8888

HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=6 \
    CMD wget -q -O /dev/null http://127.0.0.1:8888/api/health || exit 1

CMD ["/app/poprako-server"]

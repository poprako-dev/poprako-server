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
COPY poprako-obj-dept ./poprako-obj-dept
COPY poprako-obj-dept-macro ./poprako-obj-dept-macro
COPY poprako-rdb-core ./poprako-rdb-core
COPY benches ./benches

RUN mkdir -p src && \
    echo 'fn main() {}' > src/main.rs && \
    echo '' > src/lib.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    CARGO_INCREMENTAL=1 \
    cargo build --locked --release --bin poprako-server && \
    rm -rf src

# Rebuild with actual source, reusing the dependency artifacts from the layer
# above.
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    CARGO_INCREMENTAL=1 \
    cargo clean --package poprako-server && \
    cargo build --locked --release --bin poprako-server && \
    cp /work/target/release/poprako-server /work/poprako-server

FROM alpine:3.22 AS runtime

WORKDIR /app

LABEL org.opencontainers.image.source="https://github.com/poprako-dev/poprako-server"

RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    libpq && \
    addgroup -S poprako && \
    adduser -S -G poprako -h /app poprako

COPY --from=builder --chown=poprako:poprako \
    /work/poprako-server /app/poprako-server
COPY --chown=poprako:poprako \
    deploy/poprako-server/app_config.toml /app/app_config.toml

USER poprako

EXPOSE 8888

HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=6 \
    CMD wget -q -O /dev/null http://127.0.0.1:8888/api/health || exit 1

CMD ["/app/poprako-server"]

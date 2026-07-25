FROM rust:1-alpine3.22 AS builder

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
COPY migrations ./migrations
COPY benches ./benches

RUN mkdir -p src/bin && \
    echo 'fn main() {}' > src/main.rs && \
    echo 'fn main() {}' > src/bin/poprako_db_bridge.rs && \
    echo 'fn main() {}' > src/bin/poprako-db-migrate.rs && \
    echo '' > src/lib.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    CARGO_INCREMENTAL=1 \
    cargo build --release --bins && \
    rm -rf src

# Rebuild with actual source, reusing the dependency artifacts from the layer
# above.
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    CARGO_INCREMENTAL=1 \
    cargo clean --package poprako-server && \
    cargo build --release --bins && \
    cp /work/target/release/poprako-server /work/poprako-server && \
    cp /work/target/release/poprako-db-bridge /work/poprako-db-bridge && \
    cp /work/target/release/poprako-db-migrate /work/poprako-db-migrate

FROM alpine:3.22 AS runtime

WORKDIR /app

RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    libpq \
    postgresql-client

COPY --from=builder /work/poprako-server /app/poprako-server
COPY --from=builder /work/poprako-db-bridge /app/poprako-db-bridge
COPY --from=builder /work/poprako-db-migrate /app/poprako-db-migrate
COPY deploy/poprako-server/application_config.json /app/application_config.json

EXPOSE 8888

CMD ["/app/poprako-server"]

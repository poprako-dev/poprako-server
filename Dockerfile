FROM rust:1-alpine3.22 AS builder

WORKDIR /work

RUN apk add --no-cache \
    build-base \
    cmake \
    curl \
    libpq-dev \
    perl \
    pkgconf

COPY Cargo.toml Cargo.lock ./
COPY poprako-util ./poprako-util
COPY migrations ./migrations
COPY src ./src

RUN cargo build --release --bin poprako-server

FROM alpine:3.22 AS runtime

WORKDIR /app

RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    libpq

COPY --from=builder /work/target/release/poprako-server /app/poprako-server
COPY deploy/poprako-server/application_config.json /app/application_config.json

EXPOSE 8888

CMD ["/app/poprako-server"]

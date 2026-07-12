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
COPY poprako-transactional ./poprako-transactional
COPY poprako-util ./poprako-util
COPY src ./src

RUN cargo build --release --bin poprako-r

FROM alpine:3.22 AS runtime

WORKDIR /app

RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    libpq

COPY --from=builder /work/target/release/poprako-r /app/poprako-r
COPY deploy/poprako-sr/application_config.json /app/application_config.json

EXPOSE 8888

CMD ["/app/poprako-r"]

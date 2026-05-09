FROM rust:1.87-slim-bookworm AS builder
WORKDIR /app
ENV CARGO_NET_RETRY=10 \
    CARGO_HTTP_TIMEOUT=120 \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=git

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo fetch --locked && cargo build --release --locked

FROM debian:13-slim
WORKDIR /app
COPY --from=builder /app/target/release/api /usr/local/bin/api
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/api"]

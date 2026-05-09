FROM rust:1.87-slim-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM debian:13-slim
WORKDIR /app
COPY --from=builder /app/target/release/api /usr/local/bin/api
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/api"]

FROM rust:1.87-slim-bookworm AS builder
WORKDIR /app
ENV CARGO_NET_RETRY=10 \
    CARGO_HTTP_TIMEOUT=120 \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=git

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY resources ./resources

RUN cargo build --release --locked --bin build_index
RUN REFERENCES_FILE=/app/resources/references.json.gz INDEX_FILE=/app/resources/index.bin \
    ./target/release/build_index
RUN cargo build --release --locked --bin api

FROM debian:13-slim
WORKDIR /app
COPY --from=builder /app/target/release/api /usr/local/bin/api
COPY --from=builder /app/resources /app/resources
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/api"]

FROM rust:1.92-oracle AS builder
# Image name still contains "oracle" (company Rust toolchain); the app does not use Oracle DB.

WORKDIR /app

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    CARGO_NET_GIT_FETCH_WITH_CLI=true

# Layer 1: dependency cache (invalidates only when Cargo.lock changes)
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/notification_consumer.rs && \
    cargo build --release --locked && \
    rm -f target/release/data-notification target/release/notification-consumer \
    target/release/deps/data_notification* target/release/deps/notification_consumer*

# Layer 2: application source (rebuilds only app code)
COPY src ./src
RUN cargo build --release --locked --bin data-notification --bin notification-consumer && \
    install -m 0755 target/release/data-notification /tmp/data-notification && \
    install -m 0755 target/release/notification-consumer /tmp/notification-consumer

FROM debian:oracle-slim-v2 AS runtime

WORKDIR /app

COPY --from=builder /tmp/data-notification ./data-notification
COPY --from=builder /tmp/notification-consumer ./notification-consumer
COPY templates ./templates

EXPOSE 8080 9090

CMD ["./data-notification"]

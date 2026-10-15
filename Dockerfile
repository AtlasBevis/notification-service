FROM rust:1.92-bookworm AS builder

WORKDIR /app

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    CARGO_NET_GIT_FETCH_WITH_CLI=true

# Layer 1: dependency cache (invalidates only when Cargo.lock changes)
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/notification_job.rs && \
    cargo build --release --locked && \
    rm -f target/release/data-notification target/release/notification-job \
    target/release/deps/data_notification* target/release/deps/notification_job*

# Layer 2: application source (rebuilds only app code)
# build.rs compiles proto/ into the gRPC code used by src/grpc.
COPY build.rs ./
COPY proto ./proto
COPY src ./src
RUN cargo build --release --locked --bin data-notification --bin notification-job && \
    install -m 0755 target/release/data-notification /tmp/data-notification && \
    install -m 0755 target/release/notification-job /tmp/notification-job

FROM debian:bookworm-slim AS runtime

WORKDIR /app

COPY --from=builder /tmp/data-notification ./data-notification
COPY --from=builder /tmp/notification-job ./notification-job

EXPOSE 8080 9090 50051

CMD ["./data-notification"]

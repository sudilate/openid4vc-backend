FROM rust:1.88-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.toml
COPY src src
COPY config config
COPY policy policy

RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/openid4vc-backend /usr/local/bin/openid4vc-backend
COPY config config
COPY policy policy

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/openid4vc-backend"]

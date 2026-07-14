FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --locked --release -p indexlink-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --create-home --uid 10001 indexlink
WORKDIR /app
COPY --from=builder /app/target/release/indexlink-server /usr/local/bin/indexlink-server
COPY --from=builder /app/migrations /app/migrations
USER indexlink
EXPOSE 8080
ENTRYPOINT ["indexlink-server"]

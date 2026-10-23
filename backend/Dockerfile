# syntax=docker/dockerfile:1.7

FROM rust:1.99-bookworm AS build
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo fetch --locked

COPY src ./src
COPY migrations ./migrations
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp /app/target/release/levant-score-api /tmp/levant-score-api

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /tmp/levant-score-api /usr/local/bin/levant-score-api
EXPOSE 8080
ENTRYPOINT ["levant-score-api"]

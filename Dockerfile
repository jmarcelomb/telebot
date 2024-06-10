# syntax=docker/dockerfile:1
FROM rust:latest AS builder
WORKDIR /app

COPY Cargo.toml .
COPY src src

RUN cargo build --release

RUN strip target/release/telebot

FROM debian:stable-slim
WORKDIR /app
RUN apt update \
    && apt install -y openssl ca-certificates \
    && apt clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

WORKDIR /app
COPY --from=builder /app/target/release/telebot .

CMD ["./telebot"]

# ---- Builder with cargo-chef layered cache ----
ARG HTTP_PROXY
ARG HTTPS_PROXY

FROM rust:slim-bookworm AS chef
ARG HTTP_PROXY
ARG HTTPS_PROXY
ENV HTTP_PROXY=${HTTP_PROXY}
ENV HTTPS_PROXY=${HTTPS_PROXY}
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/* && cargo install cargo-chef --locked

FROM chef AS planner
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN mkdir -p crates/crawler/src crates/bot-core/src crates/napcat-sdk/src \
    && echo "fn main() {}" > crates/bot-core/src/main.rs \
    && echo "" > crates/crawler/src/lib.rs \
    && echo "" > crates/napcat-sdk/src/lib.rs
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG HTTP_PROXY
ARG HTTPS_PROXY
ENV HTTP_PROXY=${HTTP_PROXY}
ENV HTTPS_PROXY=${HTTPS_PROXY}
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release -p bot-core

# ---- Runtime with ffmpeg + chromium ----
FROM debian:bookworm-slim AS runtime
ARG HTTP_PROXY
ARG HTTPS_PROXY
ENV HTTP_PROXY=${HTTP_PROXY}
ENV HTTPS_PROXY=${HTTPS_PROXY}
WORKDIR /app

ENV TZ=Asia/Shanghai
RUN ln -sf /usr/share/zoneinfo/Asia/Shanghai /etc/localtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    ffmpeg \
    chromium \
    chromium-sandbox \
    fonts-noto-cjk \
    fonts-noto-color-emoji \
    && rm -rf /var/lib/apt/lists/*

RUN ln -s /usr/bin/chromium /usr/bin/chromium-browser 2>/dev/null || true

COPY --from=builder /app/target/release/archetto .
COPY data/logo/ data/logo/

EXPOSE 3002
CMD ["./archetto"]

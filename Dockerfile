FROM rust:1.59-slim-bullseye as builder
RUN apt-get update && apt-get install -y \
      --no-install-recommends \
      librust-openssl-dev \
      glibc-source \
      pkg-config \
      && apt-get clean \
      && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release --workspace

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y \
      --no-install-recommends \
      openssl \
      ca-certificates \
      glibc-source \
      && apt-get clean \
      && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/clearurl_bot /usr/bin/
ENTRYPOINT ["/usr/bin/clearurl_bot"]

FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release --workspace

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y \
      --no-install-recommends \
      ca-certificates \
      && apt-get clean \
      && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/clearurl_bot /usr/bin/
ENTRYPOINT ["/usr/bin/clearurl_bot"]

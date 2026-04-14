FROM rust:1.85-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src
COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs && cargo build --release && strip target/release/blog-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
RUN groupadd -r app && useradd -r app -g app
COPY --from=builder --chown=app:app /app/target/release/blog-api /usr/local/bin/
COPY --from=builder /app/migrations /app/migrations
USER app
EXPOSE 3000
CMD ["blog-api"]

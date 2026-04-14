FROM rust:1.94-slim-bookworm AS builder
WORKDIR /app

# Copy manifests for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source files for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs
RUN cargo build --release && rm -rf src

# Copy real source code
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx

# Build with SQLx offline mode
ENV SQLX_OFFLINE=true
RUN touch src/main.rs src/lib.rs && cargo build --release && strip target/release/blog-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
RUN groupadd -r app && useradd -r app -g app
COPY --from=builder --chown=app:app /app/target/release/blog-api /usr/local/bin/
COPY --from=builder /app/migrations /app/migrations
USER app
EXPOSE 3000
CMD ["blog-api"]

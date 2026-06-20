# Use the official Rust image.
FROM --platform=linux/amd64 rust:1.93-bookworm AS builder

# Copy local code to the container image.
WORKDIR /usr/src/rsky
COPY Cargo.toml Cargo.lock rust-toolchain ./
COPY db-schema db-schema
COPY lexicon lexicon
COPY identity identity
COPY syntax syntax
COPY crypto crypto
COPY feedgen feedgen
COPY janitor janitor
COPY jetstream jetstream

# Install production dependencies and build a release artifact.
RUN cargo build --release --package jetstream

FROM --platform=linux/amd64 debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/app

COPY --from=builder /usr/src/rsky/target/release/jetstream .

CMD ["./jetstream"]
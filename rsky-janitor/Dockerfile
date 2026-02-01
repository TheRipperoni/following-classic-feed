# Use the official Rust image.
FROM --platform=linux/amd64 rust AS builder

# Copy local code to the container image.
WORKDIR /usr/src/rsky
COPY Cargo.toml Cargo.lock rust-toolchain ./
COPY rsky-lexicon rsky-lexicon
COPY rsky-identity rsky-identity
COPY rsky-syntax rsky-syntax
COPY rsky-crypto rsky-crypto
COPY rsky-feedgen rsky-feedgen
COPY rsky-janitor rsky-janitor
COPY rsky-jetstream rsky-jetstream

# Install production dependencies and build a release artifact.
RUN cargo build --release --package rsky-janitor

FROM --platform=linux/amd64 debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/app

COPY --from=builder /usr/src/rsky/target/release/rsky-janitor .

CMD ["./rsky-janitor"]
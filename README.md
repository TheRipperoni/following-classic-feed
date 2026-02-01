# RSky

[![Ceasefire Now](https://badge.techforpalestine.org/default)](https://techforpalestine.org/learn-more) [![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

`rsky` is a Rust-based implementation of various AT Protocol (atproto) components, including a feed generator, a Jetstream subscriber, and a database janitor.

## Overview

RSky provides a set of tools for interacting with the AT Protocol:
- **Feed Generator**: Mimics the behavior of the following feed prior to updated changes and supports additional feature customization.
- **Jetstream Subscriber**: Subscribes to the AT Protocol event stream (firehose) and processes records.
- **Janitor**: A database cleanup service that maintains data retention policies.

The project is structured as a Cargo workspace with several crates providing cryptographic helpers, identity resolution, lexicon types, and syntax parsing.

## Project Structure

- `rsky-crypto`: Cryptographic helpers (secp256k1, p256, multibase).
- `rsky-feedgen`: Rocket-based feed generator implementation and main API.
- `rsky-identity`: DID and handle resolution logic.
- `rsky-jetstream`: Subscriber for the Jetstream firehose.
- `rsky-lexicon`: Central place for protocol-specific data structures and lexicons.
- `rsky-syntax`: Parsing logic for AT URIs and other protocol syntax.
- `rsky-janitor`: Cleanup service for the database.

## Requirements

### Prerequisites
- **Rust**: Latest stable Rust toolchain.
- **PostgreSQL**: Used for data storage (PostgreSQL 14 recommended).
- **Diesel CLI**: Required for database migrations.
  ```bash
  cargo install diesel_cli --no-default-features --features postgres
  ```
- **Docker & Docker Compose**: For containerized deployment.

## Setup & Installation

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/trentoncoleman/rsky.git
    cd rsky
    ```

2.  **Environment Setup**:
    Create `.env` files for the services. You can start by copying from the environment variables section below. The project uses:
    - `postgres.env`
    - `feedgen.env`
    - `janitor.env`
    - `jetstream.env`

3.  **Database Migrations**:
    Navigate to the feed generator directory and run migrations:
    ```bash
    cd rsky-feedgen
    diesel migration run
    ```

## Environment Variables

### Database (`postgres.env`)
- `POSTGRES_USER`: Database user.
- `POSTGRES_PASSWORD`: Database password.
- `POSTGRES_DB`: Database name.

### Feed Generator (`feedgen.env`)
- `DATABASE_URL`: Connection string for the PostgreSQL database.
- `READ_REPLICA_URL`: Connection string for the read replica (optional, defaults to `DATABASE_URL`).
- `FEEDGEN_SERVICE_DID`: The DID of the feed generator service.
- `FEEDGEN_HOSTNAME`: The hostname where the feed generator is hosted.
- `FEEDGEN_SUBSCRIPTION_ENDPOINT`: WebSocket endpoint for the subscription (e.g., `wss://jetstream1.us-west.bsky.network`).

### Janitor (`janitor.env`)
- `DATABASE_URL`: Connection string for the PostgreSQL database.
- `CRON_SCHEDULE`: Cron expression for cleaning intervals (default: `0 0 0 * * * *`).
- `RETENTION_DAYS`: Number of days to keep posts/likes/reposts (default: `2`).

### Jetstream (`jetstream.env`)
- `DATABASE_URL`: Connection string for the PostgreSQL database.
- `WANTED_COLLECTIONS`: Filter for collections to subscribe to.

## Running the Services

### Using Docker Compose (Recommended)
To start the entire stack (Database, Feedgen, Janitor, Jetstream):
```bash
docker-compose up -d
```

### Running Manually
You can run each component using `cargo`:
```bash
# Run Feed Generator
cargo run -p rsky-feedgen

# Run Janitor
cargo run -p rsky-janitor

# Run Jetstream Subscriber
cargo run -p rsky-jetstream
```

## Testing

Tests are implemented using standard Rust `#[test]` attributes.

- **Run all tests**:
  ```bash
  cargo test
  ```
- **Run tests for a specific crate**:
  ```bash
  cargo test -p rsky-syntax
  ```

## CI/CD and Deployment

This project uses GitHub Actions for CI/CD. On every push to the `main` branch, it builds Docker images for `feedgen`, `janitor`, and `jetstream`, pushes them to the GitHub Container Registry (GHCR), and deploys them to a configured server.

### GitHub Secrets
The following secrets are required for deployment:
- `DEPLOY_HOST`: The IP address or hostname of your server.
- `DEPLOY_USER`: The SSH username.
- `DEPLOY_KEY`: Your private SSH key.

### Server Setup
1.  Install Docker and Docker Compose on your server.
2.  Create the deployment directory: `mkdir -p ~/rsky-deploy/`.
3.  Ensure your `.env` files are present in `~/rsky-deploy/` on the server.

## License

This project is licensed under the Apache License 2.0. See the `LICENSE` file for details.

## Credits
Based on the original [RSky](https://github.com/blacksky-algorithms/rsky).
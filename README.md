# Following Classic Feed

## Overview

Source code for the Following Classic Feed project.

## Project Structure

- `crypto`: Cryptographic helpers (secp256k1, p256, multibase).
- `feedgen`: Axum-based feed generator implementation and main API.
- `identity`: DID and handle resolution logic.
- `jetstream`: Subscriber for the Jetstream firehose.
- `lexicon`: Central place for protocol-specific data structures and lexicons.
- `syntax`: Parsing logic for AT URIs and other protocol syntax.
- `janitor`: Cleanup service for the database.
- `stats-viz`: Web frontend for viewing statistics.

## Requirements

### Prerequisites
- **Rust**: Latest stable Rust toolchain.
- **PostgreSQL**: Used for data storage (PostgreSQL 16 required).
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
    cd feedgen
    diesel migration run
    ```

## Environment Variables

### Database (`postgres.env`)
- `POSTGRES_USER`: Database user.
- `POSTGRES_PASSWORD`: Database password.
- `POSTGRES_DB`: Database name.

### Feed Generator (`feedgen.env`)
- `DATABASE_URL`: Connection string for the PostgreSQL database (write pool).
- `READ_REPLICA_URL`: Connection string for the read replica (optional, defaults to `DATABASE_URL`).
- `API_KEY`: Pre-shared API key for internal endpoint authentication.
- `PORT`: Service port (default: `8000`).
- `JWT_SECRET`: Secret key for HS256 JWT signing.
- `USERNAME`/`PASSWORD`: Basic auth credentials.
- `FEEDGEN_SERVICE_DID`: The DID of the feed generator service.
- `FEEDGEN_HOSTNAME`: The hostname where the feed generator is hosted.
- `FEEDGEN_DOMAIN`: Domain for Caddy to route to feedgen (e.g., `feedgen.example.com`).
- `STATS_DOMAIN`: Domain for Caddy to route to stats-viz (e.g., `stats.example.com`).
- `FEEDGEN_SUBSCRIPTION_ENDPOINT`: WebSocket endpoint for the subscription (e.g., `wss://jetstream1.us-west.bsky.network`).
- `ENABLE_BACKFILL`: Set to `true` to enable the backfill worker (default: disabled).
- `WRITE_POOL_SIZE`: Database write connection pool size (default: `40`).
- `READ_POOL_SIZE`: Database read connection pool size (default: `40`).

### Janitor (`janitor.env`)
- `DATABASE_URL`: Connection string for the PostgreSQL database.
- `CRON_SCHEDULE`: Cron expression for cleaning intervals (default: `0 0 * * * *`).
- `RETENTION_DAYS`: Number of days to keep posts/likes/reposts (default: `2`).
- `PORT`: Health endpoint port (default: `8001`).

### Jetstream (`jetstream.env`)
- `DATABASE_URL`: Connection string for the PostgreSQL database.
- `WANTED_COLLECTIONS`: Filter for collections to subscribe to.

## Running the Services

### Reverse Proxy (Caddy)
Traffic is directed to `feedgen` and `stats-viz` via Caddy. Ensure `FEEDGEN_DOMAIN` and `STATS_DOMAIN` environment variables are set.

### Using Docker Compose (Recommended)
To start the entire stack (Database, Feedgen, Janitor, Jetstream):
```bash
docker-compose up -d
```

### Running Manually
You can run each component using `cargo`:
```bash
# Run Feed Generator
cargo run -p feedgen

# Run Janitor
cargo run -p janitor

# Run Jetstream Subscriber
cargo run -p jetstream
```

## Testing

Tests are implemented using standard Rust `#[test]` attributes.

- **Run all tests**:
  ```bash
  cargo test
  ```
- **Run tests for a specific crate**:
  ```bash
  cargo test -p syntax
  ```

## CI/CD and Deployment

This project uses GitHub Actions for CI/CD. On every push to the `main` branch, it builds Docker images for `feedgen`, `janitor`, and `jetstream`, pushes them to the GitHub Container Registry (GHCR), and deploys them to a configured server.

### GitHub Secrets
The following secrets are required for deployment:
- `DEPLOY_HOST`: The IP address or hostname of your server.
- `DEPLOY_USER`: The SSH username.
- `DEPLOY_KEY`: Your private SSH key.
- `DEPLOY_PASSWORD`: (Optional) Password for SSH authentication.

### Server Setup
1.  Install Docker and Docker Compose on your server.
2.  Create the deployment directory: `mkdir -p ~/deploy/`.
3.  Ensure your `.env` files are present in `~/deploy/` on the server.

For detailed development documentation (build instructions, testing, code style), see `.junie/AGENTS.md`.

## License

This project is licensed under the Apache License 2.0. See the `LICENSE` file for details.

## Credits
Based on the original [RSky](https://github.com/blacksky-algorithms/rsky).
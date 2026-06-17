# AGENTS.md — rsky Project Guide

## Build/Configuration Instructions

### Prerequisites

- **Rust toolchain**: This project pins `channel = "1.93.0"` in `rust-toolchain` (components: `clippy`, `rustfmt`). Use `rustup` to install it.
- **PostgreSQL 16**: Required at runtime by all services (feedgen, janitor, jetstream). Docker Compose provides a `postgres:16-bookworm` image.
- **Diesel CLI**: Install with `cargo install diesel_cli --no-default-features --features postgres` (needed for running migrations).

### Environment Variables

Each service loads its own `.env` file from the workspace root:

| Service | Env File | Key Variables |
|---------|----------|--------------|
| `feedgen` | `feedgen.env` | `DATABASE_URL`, `READ_REPLICA_URL`, `API_KEY`, `PORT`, `FEEDGEN_HOSTNAME`, `FEEDGEN_SERVICE_DID`, `JWT_SECRET`, `USERNAME`, `PASSWORD`, `ENABLE_BACKFILL` |
| `janitor` | `janitor.env` | `DATABASE_URL`, `CRON_SCHEDULE`, `RETENTION_DAYS`, `PORT` |
| `jetstream` | `jetstream.env` | `DATABASE_URL` |
| `postgres` | `postgres.env` | PostgreSQL server variables (used only by the `db` container) |

The `feedgen` crate supports separate write (`DATABASE_URL`) and read (`READ_REPLICA_URL`) database URLs. Pool sizes are configured via `WRITE_POOL_SIZE` and `READ_POOL_SIZE` (default: 40 each).

### Running Locally

```bash
# Start PostgreSQL (required by all services)
docker compose up -d db

# Run a specific service
cargo run -p feedgen
cargo run -p janitor
cargo run -p jetstream
```

### Docker Deployment

```bash
docker compose up -d
```

This starts `db` (Postgres 16), `feedgen` (port 8003:8000), `janitor`, `jetstream`, and `stats-viz` (port 8002:80).

### Build Notes

- **Diesel schema management**: A `diesel.toml` at the workspace root configures `diesel print-schema` to output to `db-schema/src/lib.rs`, with migrations in `feedgen/migrations/`.
- **Docker builds**: A `.dockerignore` file excludes `target/`, `postgres/`, `.git/`, and other build artifacts from the Docker context to speed up builds.

- Workspace `resolver = "2"`.
- Release profile: `debug = 2` (line numbers preserved for backtraces on crash).
- The `syntax` crate has no external runtime dependency (pure parsing).
- The `crypto` crate uses `secp256k1` (with `global-context`, `serde`, `rand`, `hashes` features) and `p256` for ECDSA over secp256k1 and P-256 respectively, plus `multibase` for encoding.

## Testing Information

### Running Tests

```bash
# All workspace tests
cargo test

# A specific crate
cargo test -p syntax
cargo test -p crypto

# A specific test by fully-qualified name
cargo test -p syntax --lib aturi::tests::test_at_uri_did_hostname

# Pattern matching on test names
cargo test -p crypto verify
```

### Adding New Tests

Tests live at the bottom of the relevant source file inside `#[cfg(test)] mod tests`. Example from `syntax/src/aturi.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_at_uri_did_hostname() {
        let uri = AtUri::new(
            "at://did:plc:abc/app.bsky.feed.post/123".to_string(),
            None,
        )
        .unwrap();
        assert_eq!(uri.get_hostname(), "did:plc:abc");
        assert_eq!(uri.get_collection(), "app.bsky.feed.post");
        assert_eq!(uri.get_rkey(), "123");
    }
}
```

Run with:
```bash
cargo test -p syntax --lib aturi::tests::test_at_uri_did_hostname
```

> **Note:** Crate-namespace tests are registered per-file in `lib.rs` (e.g., `syntax/src/lib.rs` has `pub mod aturi;`). Not all crates use this pattern — check the crate's `lib.rs` for its module structure.

### Test Patterns Observed

- **`syntax` crate (aturi.rs)**: 27 tests, all synchronous. Test `AtUri` parsing, construction, setters, base-URI resolution, invalid inputs, and regex helpers.
- **`crypto` crate**: 52 tests across submodules (`verify`, `multibase`, `did`, `p256::*`, `secp256k1::*`, `utils`). Tests cover signature verification, DID key parse/format roundtrips, pubkey compress/decompress roundtrips, edge cases (empty/invalid inputs).
- **`feedgen` crate (auth.rs)**: JWT verification tests for ES256K, ES256, and HS256 algorithms.

## Additional Development Information

### Workspace & Crate Dependencies

```yaml
crypto:      # secp256k1, p256, multibase — cryptographic helpers
db-schema:   # Diesel schema definitions (shared by feedgen, janitor, jetstream)
feedgen:     # Axum-based HTTP feed generator (was Rocket, migrated to Axum 0.8.9)
identity:    # DID & handle resolution (PLC directory, did:web, DNS)
janitor:     # Cron-based DB cleanup service
jetstream:   # Tokio-based AT Protocol Jetstream firehose subscriber
lexicon:     # AT Protocol lexicon data structures (reused across crates)
syntax:      # AT URI parsing (pure, no async/DB deps)
```

### Code Style & Patterns

- **Database access**: All services use `diesel = "=2.1.5"` with PostgreSQL + r2d2 connection pooling. The `feedgen` crate uses `deadpool-diesel` (Tokio1 runtime) instead of `rocket_sync_db_pools` — the Rocket ORM macros are no longer used.
- **Error handling**: `anyhow::Result` is the standard return type across all crates. Bail (`anyhow::bail!`) for early exits.
- **Async runtime**: `feedgen` and `jetstream` use `tokio` (full features). `feedgen` is built on `axum` 0.8 with tower-http middleware (CORS, tracing).
- **JWT verification** (`feedgen/src/auth.rs`): Supports ES256K, ES256 (via `crypto::verify`), and HS256 (via `hmac` + `sha2`). For ES256K/ES256, the signing key is extracted from either `did:key` or resolved via `IdResolver`. HS256 uses a `JWT_SECRET` env var.
- **Identity resolution** (`identity` crate): `IdResolver` wraps `HandleResolver` (DNS TXT-based) and `DidResolver` (PLC directory + did:web). Uses configurable timeouts (default 3s), caching, and optional backup DNS.
- **Metrics**: `feedgen` exposes a `/metrics` endpoint via the `prometheus` crate with custom middleware for request tracking.
- **Serialization**: `serde` + `serde_json` throughout. JWT header/payload use serde with custom camelCase field names via `#[serde(rename)]`.
- **Lexicons**: Always use the shared `lexicon` crate for AT Protocol data structures rather than defining local copies.
- **API routes** (`feedgen/src/main.rs`): XRPC endpoints (`/xrpc/app.bsky.feed.getFeedSkeleton`, `/xrpc/app.bsky.feed.describeFeedGenerator`), internal endpoints (`/queue/*`, `/cursor`, `/stats`, `/visitors`, `/health`, `/janitor/config`, `/user_feed_preference`, `/following_preferences`), well-known DID (`/.well-known/did.json`), and `/metrics`.
- **Jetstream** (`jetstream` crate): Connects to the AT Protocol Jetstream firehose via WebSocket, processes create/delete operations, and writes to PostgreSQL. Uses a queue-based approach with `tokio` tasks.
- **Janitor** (`janitor` crate): Cron-scheduled database cleanup — deletes posts, reposts, and likes older than `RETENTION_DAYS`.

### Recent Improvements

- **SQL injection fix** (`algo.rs`): `mutuals_query_str` now uses `sanitize_did()` before DID interpolation, matching the pattern used in `post_query_str`.
- **Cursor constant** (`algo.rs`): Hardcoded `230 * 1_000_000` replaced with named constant `CURSOR_TIMESTAMP_TOLERANCE_NS` (type `u32` for chrono compatibility).
- **Dead iterator pattern fixed** (`algo.rs`): `results.clone().into_iter().map(...).for_each(drop)` replaced with `for` loops in both `get_posts_by_following_media` and `get_posts_by_mutuals`, removing unnecessary `.clone()`.
- **`build_uri` helper** (`processor.rs`): Deduplicated 8 repeated `format!("at://{did}/{collection}/{rkey}")` patterns into a single helper function.
- **`println!` → `tracing::info!`** (`jetstream.rs`): Production logging now uses structured tracing instead of raw println.
- **`panic!()` → graceful skip** (`agent.rs`): Missing `subject`/`createdAt` fields on follow records now log a warning and skip instead of panicking.
- **Max retry cap** (`backfill.rs`): Backfill jobs that fail 10+ times are marked as `failed` instead of retrying infinitely.
- **`extern crate` cleanup**: Removed unnecessary `extern crate serde`, `extern crate serde_json`, `extern crate lexicon`, `extern crate url` from `feedgen/src/lib.rs`, `jetstream/src/lib.rs`, and `identity/src/lib.rs` (Rust 2021 edition makes them unnecessary).
- **`.gitignore` simplification**: Replaced ~1070 individual PostgreSQL data directory entries with a single `/postgres/` wildcard.
- **`.gitignore` additions**: Added `stats-viz/dist/` and `stats-viz/node_modules/` entries to prevent accidental commits of build artifacts.
- **`.dockerignore`**: Created to exclude `target/`, `postgres/`, `.git/`, `.cargo/`, and other large files from Docker build context.
- **`diesel.toml`**: Created configuration file for Diesel CLI, mapping `db-schema/src/lib.rs` for schema output and `feedgen/migrations` for migrations directory.
- **DB URL validation** (`main.rs`): Changed `unwrap_or_default()` to `expect()` with descriptive messages for `DATABASE_URL` and `READ_REPLICA_URL`, catching missing config at startup with a clear error.
- **Backfill gating** (`main.rs`): Backfill worker now only spawns when `ENABLE_BACKFILL=true` is set (default: disabled), saving a Tokio task slot.
- **Janitor error resilience** (`janitor/src/main.rs`): Replaced `expect()` panics in `get_config()` and `clean_db()` with `Result` returns. Missing/empty config or DB errors are now logged and retried instead of crashing the process.
- **Janitor PORT config** (`janitor/src/main.rs`): Health endpoint now reads `PORT` env var (default: `8001`), matching the pattern used by feedgen.
- **Janitor cron parsing** (`janitor/src/main.rs`): Invalid cron schedule falls back to hourly (`0 0 * * * *`) with a logged warning instead of panicking.
- **Error type consistency** (`queue.rs`, `db/mod.rs`): Changed `queue_creation`, `queue_deletion`, `user_config_creation`, `user_config_update`, and `following_pref_update` from `Result<(), String>` to `anyhow::Result<()>` — consistent with the rest of the project.
- **Workspace dependency management** (`Cargo.toml`): Added commonly-shared external crates (`anyhow`, `chrono`, `serde`, `serde_json`, `tokio`, `tracing`, `tracing-subscriber`, `dotenvy`, `axum`) to `[workspace.dependencies]`. All crate-level `Cargo.toml` files reference workspace versions, ensuring single-source versioning.
- **Query parameter validation** (`handlers/config.rs`, `handlers/algo.rs`): Missing required params (`service`, `did`) now return `400 Bad Request` with a clear error message instead of silently passing empty strings to DB functions.
- **`serde_derive` cleanup** (4 crates): Removed all redundant `serde_derive` dependencies and `extern crate` declarations. The `serde = { features = ["derive"] }` workspace dep provides derive macros directly; no separate `serde_derive` crate needed.
- **Workspace deps for lexicons** (`syntax/Cargo.toml`, `lexicon/Cargo.toml`): Switched `anyhow`, `serde`, `serde_json`, and `chrono` to use `{ workspace = true }` for consistent versioning across all workspace crates.
- **Cursor parsing extracted** (`algo.rs`): Created `apply_cursor_to_single_query()` helper, eliminating duplicated inline cursor parsing logic in `get_posts_by_following_media` and `get_posts_by_mutuals` (~30 lines removed per function).
- **npm dep classification** (`stats-viz/package.json`): Moved `@types/react`, `@types/react-dom`, `@vitejs/plugin-react`, and `vite` from `dependencies` to `devDependencies` — these are build-time/type-only, never bundled.
- **DB healthcheck** (`docker-compose.yml`): Added `pg_isready` healthcheck to the `db` service; all dependent services (`feedgen`, `janitor`, `jetstream`) now use `condition: service_healthy` to wait for PostgreSQL readiness.
- **README update**: Changed "Rocket-based" → "Axum-based", "PostgreSQL 14" → "PostgreSQL 16 required", added missing env vars (`API_KEY`, `PORT`, `JWT_SECRET`, `USERNAME`/`PASSWORD`, `ENABLE_BACKFILL`, `WRITE_POOL_SIZE`/`READ_POOL_SIZE`, janitor `PORT`), and added a reference to `.junie/AGENTS.md`.
- **`serde` import consistency**: Added explicit `use serde::{Deserialize, Serialize};` to all model files across `feedgen` (26 files) and `jetstream` (2 files) that were previously relying only on `#[macro_use] extern crate serde_derive;`.

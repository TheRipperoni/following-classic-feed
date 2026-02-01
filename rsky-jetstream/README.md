# <h1> rsky-jetstream </h1>

<p><strong>An AT Protocol Jetstream Subscriber</strong></p>

[![dependency status](https://deps.rs/repo/github/blacksky-algorithms/rsky/status.svg?style=flat-square)](https://deps.rs/repo/github/blacksky-algorithms/rsky) [![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

`rsky-jetstream` is a high-performance Rust service designed to subscribe to the AT Protocol [Jetstream](https://github.com/bluesky-social/jetstream). It consumes events from the firehose, processes repo commits, account updates, and identity changes, and forwards relevant data to a queuing service.

### Features

- **Efficient Jetstream Consumption**: Built on `tokio` and `tokio-tungstenite` for asynchronous WebSocket communication.
- **Selective Filtering**: Supports filtering by collection (e.g., posts, likes, reposts, follows).
- **Downstream Integration**: Automatically queues create and delete operations to a configurable endpoint.
- **Modular Design**: Leverages `rsky-lexicon` for protocol-compliant data structures.

### Requirements

- **Rust**: Latest stable toolchain (v1.75+ recommended).
- **OpenSSL**: Required for secure WebSocket and HTTP connections.
- **Queuing Service**: A compatible service (like `rsky-feedgen`) to receive processed events.

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RSKY_API_KEY` | API key used for authenticating with the queue endpoint. | (Required) |
| `FEEDGEN_QUEUE_ENDPOINT` | The URL of the service where events are queued. | `http://127.0.0.1:8000` |
| `FEEDGEN_SUBSCRIPTION_ENDPOINT` | The Jetstream WebSocket endpoint. | `wss://jetstream1.us-west.bsky.network` |
| `WANTED_COLLECTIONS` | Query parameters defining which collections to subscribe to. | (Posts, Reposts, Follows, Likes) |

### Setup & Run

#### Using Cargo

1. **Clone the repository**:
   ```bash
   git clone https://github.com/blacksky-algorithms/rsky.git
   cd rsky/rsky-jetstream
   ```

2. **Configure Environment**:
   Create a `.env` file or export the variables listed above.

3. **Run the service**:
   ```bash
   cargo run --release
   ```

#### Using Docker

You can build and run the service using the provided Dockerfile:

```bash
docker build -t rsky-jetstream .
docker run --env-file .env rsky-jetstream
```

### Project Structure

- `src/main.rs`: Entry point, WebSocket client, and event routing logic.
- `src/jetstream.rs`: Data structures and parsing logic for Jetstream messages.
- `Dockerfile`: Production-ready container definition.

### Testing

Run the test suite using Cargo:

```bash
cargo test -p rsky-jetstream
```

Tests cover message parsing for various event types (commits, account status, identity updates).

### License

Licensed under the Apache License, Version 2.0. See [LICENSE](../LICENSE) for details.
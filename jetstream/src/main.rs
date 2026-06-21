use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use dotenvy::dotenv;
use futures::StreamExt as _;
use jetstream::metrics::Metrics;
use jetstream::processor::process;
use jetstream::queue::get_cursor;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::protocol::Message;

async fn health_check() -> &'static str {
    "OK"
}

async fn get_metrics(
    State(metrics): State<Arc<Metrics>>,
) -> Json<jetstream::metrics::MetricsSnapshot> {
    Json(metrics.snapshot())
}

#[tracing::instrument]
#[tokio::main]
async fn main() {
    dotenv().ok();
    let metrics = Arc::new(Metrics::new());
    let default_subscriber_path = env::var("FEEDGEN_SUBSCRIPTION_ENDPOINT")
        .unwrap_or("wss://jetstream1.us-west.bsky.network".into());
    let wanted_collections = env::var("WANTED_COLLECTIONS")
        .unwrap_or("wantedCollections=app.bsky.feed.post&wantedCollections=app.bsky.feed.repost&wantedCollections=app.bsky.graph.follow&wantedCollections=app.bsky.feed.like".into());
    let client = reqwest::Client::new();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let skip_cursor = env::var("SKIP_CURSOR")
        .unwrap_or("false".into())
        .to_lowercase();
    let skip_cursor = skip_cursor == "true" || skip_cursor == "1" || skip_cursor == "yes";

    let mut cursor: Option<i64> = None;

    let queue_endpoint =
        env::var("FEEDGEN_QUEUE_ENDPOINT").unwrap_or("http://127.0.0.1:8000".into());
    let cursor_endpoint = format!("{}/cursor", queue_endpoint);

    if !skip_cursor {
        match get_cursor(
            cursor_endpoint.clone(),
            default_subscriber_path.clone(),
            &client,
        )
        .await
        {
            Ok(state) => {
                tracing::info!("Starting from cursor: {}", state.cursor);
                cursor = Some(state.cursor);
            }
            Err(e) => {
                tracing::warn!("Could not fetch last cursor: {}. Starting from live.", e);
            }
        }
    }

    let concurrency_limit = env::var("MAX_CONCURRENCY")
        .unwrap_or("32".into())
        .parse::<usize>()
        .unwrap_or(32);

    let metrics_addr: SocketAddr = env::var("METRICS_ADDR")
        .unwrap_or("0.0.0.0:3000".into())
        .parse()
        .expect("Invalid METRICS_ADDR");

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(get_metrics))
        .with_state(metrics.clone());

    tokio::spawn(async move {
        tracing::info!("Metrics server listening on {}", metrics_addr);
        let listener = tokio::net::TcpListener::bind(&metrics_addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    loop {
        let mut url = format!(
            "{sub}/subscribe?{filter}",
            sub = default_subscriber_path,
            filter = wanted_collections
        );
        if let Some(c) = cursor {
            url.push_str(&format!("&cursor={}", c));
        }

        match tokio_tungstenite::connect_async(url.as_str()).await {
            Ok((socket, _response)) => {
                tracing::info!("Connected to {default_subscriber_path:?}.");

                let queue_path = queue_endpoint.clone();
                let sub_path = default_subscriber_path.clone();

                socket
                    .filter_map(|msg| async move {
                        match msg {
                            Ok(Message::Text(text)) => Some(text),
                            _ => None,
                        }
                    })
                    .for_each_concurrent(concurrency_limit, |message| {
                        let client = client.clone();
                        let queue_path = queue_path.clone();
                        let sub_path = sub_path.clone();
                        let metrics = metrics.clone();
                        async move {
                            // Convert Utf8Bytes to String once here
                            let msg = message.to_string();
                            process(msg, &client, &queue_path, &sub_path, skip_cursor, metrics)
                                .await;
                        }
                    })
                    .await;
            }
            Err(error) => {
                tracing::error!("Error connecting to {default_subscriber_path:?}. Waiting to reconnect: {error:?}");
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
        }
    }
}

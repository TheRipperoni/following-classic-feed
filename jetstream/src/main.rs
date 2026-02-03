use dotenvy::dotenv;
use futures::StreamExt as _;
use jetstream::processor::process;
use jetstream::queue::get_cursor;
use std::env;
use std::time::Duration;
use tokio_tungstenite::tungstenite::protocol::Message;

#[tracing::instrument]
#[tokio::main]
async fn main() {
    dotenv().ok();
    let default_subscriber_path = env::var("FEEDGEN_SUBSCRIPTION_ENDPOINT")
        .unwrap_or("wss://jetstream1.us-west.bsky.network".into());
    let wanted_collections = env::var("WANTED_COLLECTIONS")
        .unwrap_or("wantedCollections=app.bsky.feed.post&wantedCollections=app.bsky.feed.repost&wantedCollections=app.bsky.graph.follow&wantedCollections=app.bsky.feed.like".into());
    let client = reqwest::Client::new();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut cursor: Option<i64> = None;

    let queue_endpoint =
        env::var("FEEDGEN_QUEUE_ENDPOINT").unwrap_or("http://127.0.0.1:8000".into());
    let cursor_endpoint = format!("{}/cursor", queue_endpoint);

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

    let concurrency_limit = env::var("MAX_CONCURRENCY")
        .unwrap_or("32".into())
        .parse::<usize>()
        .unwrap_or(32);

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
                        async move {
                            // Convert Utf8Bytes to String once here
                            let msg = message.to_string();
                            process(msg, &client, &queue_path, &sub_path).await;
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

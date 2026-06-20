use axum::{routing::get, Json, Router};
use chrono::Utc;
use cron::Schedule;
use dotenvy::dotenv;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{env, time::Duration};
use tokio_postgres::{Client, NoTls};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting Janitor");
    let database_url = env::var("DATABASE_URL").expect("Missing db_url");

    // Track database health status across the cleanup loop
    let db_healthy = Arc::new(AtomicBool::new(false));
    let health_flag = db_healthy.clone();

    // Spawn a health HTTP endpoint on a separate port
    tokio::spawn(async move {
        let app = Router::new().route(
            "/health",
            get(move || async move {
                if health_flag.load(Ordering::Relaxed) {
                    (axum::http::StatusCode::OK, Json(serde_json::json!({
                        "status": "ok",
                        "service": "janitor"
                    })))
                } else {
                    (axum::http::StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
                        "status": "error",
                        "service": "janitor",
                        "message": "Database connection unavailable"
                    })))
                }
            }),
        );

        let port: u16 = env::var("PORT")
            .unwrap_or("8001".to_string())
            .parse()
            .expect("PORT must be a valid u16");
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        tracing::info!("Janitor health endpoint listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect("Failed to bind TCP listener");
        axum::serve(listener, app)
            .await
            .expect("Server failed");
    });

    loop {
        tracing::info!("Looping");

        // Create a single database connection per cycle and reuse it for all operations
        let client = match connect_with_retry(&database_url).await {
            Ok(c) => {
                db_healthy.store(true, Ordering::Relaxed);
                c
            }
            Err(e) => {
                tracing::error!("Failed to connect to database: {}", e);
                db_healthy.store(false, Ordering::Relaxed);
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }
        };

        let (cron_schedule, retention_days) = match get_config(&client).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to fetch janitor config: {}", e);
                db_healthy.store(false, Ordering::Relaxed);
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }
        };

        let schedule = match Schedule::from_str(cron_schedule.as_str()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(
                    "Invalid cron schedule '{}': {}, falling back to hourly",
                    cron_schedule,
                    e
                );
                Schedule::from_str("0 0 * * * *").unwrap()
            }
        };

        let now = Utc::now();
        if let Some(next) = schedule.upcoming(Utc).take(1).next() {
            tracing::info!("Next run: {next}");
            let until_next = next - now;

            if until_next.num_seconds() > 0 {
                let secs = until_next.num_seconds() as u64;
                tracing::info!("Sleeping for {x} seconds", x = secs);
                tokio::time::sleep(Duration::from_secs(secs)).await;
            }
            if let Err(e) = clean_db(&client, retention_days).await {
                tracing::error!("Failed to clean database: {}", e);
            }
        }
    }
}

async fn connect_with_retry(database_url: &str) -> Result<Client, String> {
    loop {
        match tokio_postgres::connect(database_url, NoTls).await {
            Ok((client, connection)) => {
                // Spawn the connection handler to keep it alive in the background
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        tracing::error!("Connection error: {e}");
                    }
                });
                return Ok(client);
            }
            Err(e) => {
                tracing::error!("Failed to connect to database: {e}. Retrying in 5 seconds...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

async fn get_config(client: &Client) -> Result<(String, i32), Box<dyn std::error::Error>> {
    let row = client
        .query_one(
            "SELECT cron_schedule, retention_days FROM janitor_config ORDER BY updated_at DESC LIMIT 1",
            &[],
        )
        .await?;

    let cron_schedule: String = row.get(0);
    let retention_days: i32 = row.get(1);
    Ok((cron_schedule, retention_days))
}

async fn clean_db(client: &Client, retention_days: i32) -> Result<(), Box<dyn std::error::Error>> {
    client
        .execute(
            "DELETE FROM post WHERE date(\"indexedAt\") < now() - make_interval(days => $1)",
            &[&retention_days],
        )
        .await?;
    client
        .execute(
            "DELETE FROM repost WHERE date(\"indexedAt\") < now() - make_interval(days => $1)",
            &[&retention_days],
        )
        .await?;
    client
        .execute(
            "DELETE FROM \"like\" WHERE date(\"indexedAt\") < now() - make_interval(days => $1)",
            &[&retention_days],
        )
        .await?;
    Ok(())
}

use axum::{routing::get, Json, Router};
use chrono::Utc;
use cron::Schedule;
use dotenvy::dotenv;
use postgres::{Client, NoTls};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{env, time::Duration};
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

        let addr = SocketAddr::from(([0, 0, 0, 0], 8001));
        tracing::info!("Janitor health endpoint listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    loop {
        tracing::info!("Looping");

        // Create a single database connection per cycle and reuse it for all operations
        let mut client = match Client::connect(database_url.as_str(), NoTls) {
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

        let (cron_schedule, retention_days) = get_config(&mut client);

        let schedule =
            Schedule::from_str(cron_schedule.as_str()).expect("Failed to parse CRON expression");

        let now = Utc::now();
        if let Some(next) = schedule.upcoming(Utc).take(1).next() {
            tracing::info!("Next run: {next}");
            let until_next = next - now;

            if until_next.num_seconds() > 0 {
                let secs = until_next.num_seconds() as u64;
                tracing::info!("Sleeping for {x} seconds", x = secs);
                tokio::time::sleep(Duration::from_secs(secs)).await;
            }
            clean_db(&mut client, retention_days);
        }
    }
}

fn get_config(client: &mut Client) -> (String, i32) {
    let row = client
        .query_one(
            "SELECT cron_schedule, retention_days FROM janitor_config ORDER BY updated_at DESC LIMIT 1",
            &[],
        )
        .expect("Failed to fetch janitor config");

    let cron_schedule: String = row.get(0);
    let retention_days: i32 = row.get(1);
    (cron_schedule, retention_days)
}

fn clean_db(client: &mut Client, retention_days: i32) {
    client
        .execute(
            "DELETE FROM post WHERE date(\"indexedAt\") < now() - make_interval(days => $1)",
            &[&retention_days],
        )
        .expect("Failed to clean posts");
    client
        .execute(
            "DELETE FROM repost WHERE date(\"indexedAt\") < now() - make_interval(days => $1)",
            &[&retention_days],
        )
        .expect("Failed to clean reposts");
    client
        .execute(
            "DELETE FROM \"like\" WHERE date(\"indexedAt\") < now() - make_interval(days => $1)",
            &[&retention_days],
        )
        .expect("Failed to clean likes");
}

use chrono::Utc;
use cron::Schedule;
use dotenvy::dotenv;
use postgres::{Client, NoTls};
use std::str::FromStr;
use std::{env, thread};
use tracing_subscriber::EnvFilter;

fn main() {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting Janitor");
    let database_url = env::var("DATABASE_URL").expect("Missing db_url");

    loop {
        tracing::info!("Looping");
        let (cron_schedule, retention_days) = get_config(database_url.as_str());

        let schedule =
            Schedule::from_str(cron_schedule.as_str()).expect("Failed to parse CRON expression");

        let now = Utc::now();
        if let Some(next) = schedule.upcoming(Utc).take(1).next() {
            tracing::info!("Next run: {next}");
            let until_next = next - now;

            if until_next.num_seconds() > 0 {
                tracing::info!("Sleeping for {x} seconds", x = until_next.num_seconds());
                thread::sleep(until_next.to_std().unwrap());
            }
            clean_db(database_url.as_str(), retention_days);
        }
    }
}

fn get_config(database_url: &str) -> (String, i32) {
    let mut client = Client::connect(database_url, NoTls).expect("Unable to connect");
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

fn clean_db(database_url: &str, retention_days: i32) {
    let mut client = Client::connect(database_url, NoTls).expect("Unable to connect");
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

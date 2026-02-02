use chrono::Utc;
use cron::Schedule;
use dotenvy::dotenv;
use postgres::{Client, NoTls};
use std::str::FromStr;
use std::{env, thread};

fn main() {
    eprintln!("Starting Janitor");
    dotenv().ok();
    let cron_schedule = env::var("CRON_SCHEDULE").unwrap_or("0 0 0 * * * *".to_string());
    let database_url = env::var("DATABASE_URL").expect("Missing db_url");

    // How many days to retain data before deletion; default to 2 days
    let retention_days: i32 = env::var("RETENTION_DAYS")
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(2);

    let schedule =
        Schedule::from_str(cron_schedule.as_str()).expect("Failed to parse CRON expression");

    loop {
        eprintln!("Looping");
        let now = Utc::now();
        if let Some(next) = schedule.upcoming(Utc).take(1).next() {
            eprintln!("Cleaning");
            let until_next = next - now;
            eprintln!("Sleeping for {x}", x = until_next.num_hours());
            thread::sleep(until_next.to_std().unwrap());
            clean_db(database_url.as_str(), retention_days);
        }
    }
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

use crate::models::{CreateOp, DeleteOp};
use std::env;

pub async fn queue_delete(
    url: String,
    records: Vec<DeleteOp>,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("RSKY_API_KEY").map_err(|_| {
        "Pass a valid preshared token via `RSKY_API_KEY` environment variable.".to_string()
    })?;
    client
        .put(url)
        .json(&records)
        .header("X-RSKY-KEY", token)
        .header("Connection", "Keep-Alive")
        .header("Keep-Alive", "timeout=5, max=1000")
        .send()
        .await?;
    Ok(())
}

pub async fn queue_create<T: serde::ser::Serialize>(
    url: String,
    records: Vec<CreateOp<T>>,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("RSKY_API_KEY").map_err(|_| {
        "Pass a valid preshared token via `RSKY_API_KEY` environment variable.".to_string()
    })?;
    client
        .put(url)
        .json(&records)
        .header("X-RSKY-KEY", token)
        .header("Connection", "Keep-Alive")
        .header("Keep-Alive", "timeout=5, max=1000")
        .send()
        .await?;
    Ok(())
}

pub async fn update_cursor(
    url: String,
    service: String,
    sequence: &i64,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("RSKY_API_KEY").map_err(|_| {
        "Pass a valid preshared token via `RSKY_API_KEY` environment variable.".to_string()
    })?;
    let query = vec![("service", service), ("sequence", sequence.to_string())];
    client
        .put(url)
        .query(&query)
        .header("X-RSKY-KEY", token)
        .header("Accept", "application/json")
        .send()
        .await?;
    Ok(())
}

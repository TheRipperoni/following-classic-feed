use crate::models::{CreateOp, DeleteOp};
use std::env;

pub async fn queue_delete(
    url: String,
    records: Vec<DeleteOp>,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("API_KEY").map_err(|_| {
        "Pass a valid preshared token via `API_KEY` environment variable.".to_string()
    })?;
    client
        .post(url)
        .json(&records)
        .header("X-KEY", token)
        .header("Connection", "Keep-Alive")
        .header("Keep-Alive", "timeout=5, max=1000")
        .send()
        .await?;
    Ok(())
}

pub async fn queue_create<T: serde::ser::Serialize>(
    url: String,
    records: Vec<T>,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("API_KEY").map_err(|_| {
        "Pass a valid preshared token via `API_KEY` environment variable.".to_string()
    })?;
    client
        .post(url)
        .json(&records)
        .header("X-KEY", token)
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
    let token = env::var("API_KEY").map_err(|_| {
        "Pass a valid preshared token via `API_KEY` environment variable.".to_string()
    })?;
    let body = SubState {
        service,
        cursor: *sequence,
    };
    client
        .put(url)
        .json(&body)
        .header("X-KEY", token)
        .header("Accept", "application/json")
        .send()
        .await?;
    Ok(())
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug, Clone)]
pub struct SubState {
    pub service: String,
    pub cursor: i64,
}

pub async fn get_cursor(
    url: String,
    service: String,
    client: &reqwest::Client,
) -> Result<SubState, Box<dyn std::error::Error>> {
    let token = env::var("API_KEY").map_err(|_| {
        "Pass a valid preshared token via `API_KEY` environment variable.".to_string()
    })?;
    let query = vec![("service", service)];
    let resp = client
        .get(url)
        .query(&query)
        .header("X-KEY", token)
        .header("Accept", "application/json")
        .send()
        .await?;

    if resp.status().is_success() {
        let sub_state = resp.json::<SubState>().await?;
        Ok(sub_state)
    } else {
        Err(format!("Failed to get cursor: {}", resp.status()).into())
    }
}

use crate::app::bsky::embed::images::AspectRatio;
use crate::com::atproto::repo::Blob;
use serde::{Deserialize, Serialize};

/// A gallery of images embedded in a Bluesky record (eg, a post).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "$type")]
#[serde(rename = "app.bsky.embed.gallery")]
#[serde(rename_all = "camelCase")]
pub struct Gallery {
    pub items: Vec<Item>,
}

/// A single image item within a gallery embed record.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename = "app.bsky.embed.gallery#image")]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub image: Blob,
    /// Alt text description of the image, for accessibility.
    pub alt: String,
    pub aspect_ratio: Option<AspectRatio>,
}

/// The resolved view of a gallery embed.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "$type")]
#[serde(rename = "app.bsky.embed.gallery#view")]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub items: Vec<ViewItem>,
}

/// A single image item in the gallery view.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename = "app.bsky.embed.gallery#viewImage")]
#[serde(rename_all = "camelCase")]
pub struct ViewItem {
    /// Fully-qualified URL where a thumbnail of the image can be fetched.
    pub thumbnail: String,
    /// Fully-qualified URL where a large version of the image can be fetched.
    pub fullsize: String,
    /// Alt text description of the image, for accessibility.
    pub alt: String,
    pub aspect_ratio: Option<AspectRatio>,
}

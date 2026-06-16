/// Represents a single feed entry in the describeFeedGenerator response.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct FeedDescription {
    #[serde(rename = "uri")]
    pub uri: String,
}

/// Response model for the `app.bsky.feed.describeFeedGenerator` XRPC endpoint.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DescribeFeedGenerator {
    #[serde(rename = "did")]
    pub did: String,
    #[serde(rename = "feeds")]
    pub feeds: Vec<FeedDescription>,
}

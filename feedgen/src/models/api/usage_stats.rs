#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageStats {
    #[serde(rename = "totalVisits")]
    pub total_visits: i64,
    #[serde(rename = "uniqueVisitors")]
    pub unique_visitors: i64,
    #[serde(rename = "weeklyUniqueVisitors")]
    pub weekly_unique_visitors: i64,
}

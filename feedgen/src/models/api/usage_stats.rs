#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct UsageStats {
    #[serde(rename = "totalVisits")]
    pub total_visits: i64,
    #[serde(rename = "uniqueVisitors")]
    pub unique_visitors: i64,
    #[serde(rename = "weeklyUniqueVisitors")]
    pub weekly_unique_visitors: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_stats_serde() {
        let stats = UsageStats {
            total_visits: 1000,
            unique_visitors: 500,
            weekly_unique_visitors: 50,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: UsageStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats, deserialized);
    }

    #[test]
    fn test_usage_stats_default() {
        let stats = UsageStats::default();
        assert_eq!(stats.total_visits, 0);
        assert_eq!(stats.unique_visitors, 0);
        assert_eq!(stats.weekly_unique_visitors, 0);
    }

    #[test]
    fn test_usage_stats_serde_field_names() {
        let json = r#"{"totalVisits":42,"uniqueVisitors":7,"weeklyUniqueVisitors":3}"#;
        let stats: UsageStats = serde_json::from_str(json).unwrap();
        assert_eq!(stats.total_visits, 42);
        assert_eq!(stats.unique_visitors, 7);
        assert_eq!(stats.weekly_unique_visitors, 3);
    }
}

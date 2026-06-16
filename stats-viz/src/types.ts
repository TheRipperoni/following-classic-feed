export interface UsageStats {
  totalVisits: number;
  uniqueVisitors: number;
  weeklyUniqueVisitors: number;
}

export interface Visitor {
  id: number;
  did: string;
  web: string;
  visited_at: string;
  feed?: string;
}

export interface UserFeedPreference {
  did: string;
  show_replies: boolean;
  reply_filter_likes: number;
  reply_filter_followed_only: boolean;
  show_reposts: boolean;
  show_quote_posts: boolean;
  hide_seen_posts: boolean;
  hide_no_alt_text: boolean;
}

export interface JanitorConfig {
  id: number;
  cron_schedule: string;
  retention_days: number;
  updated_at: string;
}

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

export interface JanitorConfig {
  id: number;
  cron_schedule: string;
  retention_days: number;
  updated_at: string;
}

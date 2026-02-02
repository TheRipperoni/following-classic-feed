export interface UsageStats {
  totalVisits: number;
  uniqueVisitors: number;
}

export interface Visitor {
  id: number;
  did: string;
  web: string;
  visited_at: string;
  feed?: string;
}

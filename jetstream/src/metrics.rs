use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub struct Metrics {
    pub messages_processed: AtomicU64,
    pub posts_created: AtomicU64,
    pub posts_deleted: AtomicU64,
    pub reposts_created: AtomicU64,
    pub reposts_deleted: AtomicU64,
    pub likes_created: AtomicU64,
    pub likes_deleted: AtomicU64,
    pub follows_created: AtomicU64,
    pub follows_deleted: AtomicU64,
    pub errors: AtomicU64,
    pub start_time: Instant,
}

#[derive(Serialize)]
pub struct MetricsSnapshot {
    pub messages_processed: u64,
    pub posts_created: u64,
    pub posts_deleted: u64,
    pub reposts_created: u64,
    pub reposts_deleted: u64,
    pub likes_created: u64,
    pub likes_deleted: u64,
    pub follows_created: u64,
    pub follows_deleted: u64,
    pub errors: u64,
    pub uptime_seconds: u64,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            messages_processed: AtomicU64::new(0),
            posts_created: AtomicU64::new(0),
            posts_deleted: AtomicU64::new(0),
            reposts_created: AtomicU64::new(0),
            reposts_deleted: AtomicU64::new(0),
            likes_created: AtomicU64::new(0),
            likes_deleted: AtomicU64::new(0),
            follows_created: AtomicU64::new(0),
            follows_deleted: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            messages_processed: self.messages_processed.load(Ordering::Relaxed),
            posts_created: self.posts_created.load(Ordering::Relaxed),
            posts_deleted: self.posts_deleted.load(Ordering::Relaxed),
            reposts_created: self.reposts_created.load(Ordering::Relaxed),
            reposts_deleted: self.reposts_deleted.load(Ordering::Relaxed),
            likes_created: self.likes_created.load(Ordering::Relaxed),
            likes_deleted: self.likes_deleted.load(Ordering::Relaxed),
            follows_created: self.follows_created.load(Ordering::Relaxed),
            follows_deleted: self.follows_deleted.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }
}

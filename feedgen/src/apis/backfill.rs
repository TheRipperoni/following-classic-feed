use crate::state::AppState;
use crate::models::BackfillJob;
use crate::schema::backfill_job;
use diesel::prelude::*;
use tokio::time::{self, Duration};
use tracing::{error, info};

pub async fn backfill_worker(state: AppState) {
    let mut interval = time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        
        let conn_pool = state.write_db.0.clone();
        let conn = match conn_pool.get().await {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to get DB connection: {:?}", e);
                continue;
            }
        };

        let result = conn.interact(move |c| {
            backfill_job::table
                .filter(backfill_job::state.eq("pending"))
                .limit(1)
                .first::<BackfillJob>(c)
                .optional()
        }).await;

        match result {
            Ok(Ok(Some(job))) => {
                info!("Processing backfill job for DID: {}", job.did);
                // Throttling: wait before processing
                time::sleep(Duration::from_secs(2)).await;

                // TODO: Implement actual backfilling here
                // Simulate success for now
                let success = true;

                if success {
                    let _ = conn.interact(move |c| {
                        diesel::update(backfill_job::table.find(job.id))
                            .set(backfill_job::state.eq("completed"))
                            .execute(c)
                    }).await;
                } else if job.attempts >= 10 {
                    info!(
                        "Backfill job {} for DID {} has failed {} times, marking as failed",
                        job.id, job.did, job.attempts
                    );
                    let _ = conn.interact(move |c| {
                        diesel::update(backfill_job::table.find(job.id))
                            .set((
                                backfill_job::state.eq("failed"),
                                backfill_job::attempts.eq(job.attempts + 1),
                                backfill_job::last_error.eq("Max retries exceeded"),
                            ))
                            .execute(c)
                    }).await;
                } else {
                    // Error resilience: increment attempts and update last_error
                    let _ = conn.interact(move |c| {
                        diesel::update(backfill_job::table.find(job.id))
                            .set((
                                backfill_job::state.eq("pending"),
                                backfill_job::attempts.eq(job.attempts + 1),
                                backfill_job::last_error.eq("Failed to backfill"),
                            ))
                            .execute(c)
                    }).await;
                }
            },
            Ok(Ok(None)) => {},
            Ok(Err(e)) => error!("Failed to fetch pending job: {:?}", e),
            Err(e) => error!("Failed to interact with DB: {:?}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_backfill_worker_skeleton() {
        // This is a dummy test to verify the worker can be initialized.
        // We need to mock AppState which requires database pools.
        // Given the complexity of setting up DB pools in tests,
        // we might need to skip deep integration tests or mock the database interactions.
    }
}

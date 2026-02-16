use crate::models::errors::error_code::ErrorCode;
use crate::models::errors::validation_error_message_response::ValidationErrorMessageResponse;
use crate::models::{UsageStats, Visitor};
use crate::ReadReplicaConn;
use chrono::offset::Utc as UtcOffset;
use chrono::{DateTime, Duration};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::time::SystemTime;

/// Inserts a new visitor record into the database.
pub async fn add_visitor(
    user: String,
    service: String,
    requested_feed: String,
    connection: ReadReplicaConn,
) -> Result<(), String> {
    let result = connection
        .0
        .get()
        .await
        .map_err(|e| format!("Failed to get database connection: {}", e))?
        .interact(move |conn: &mut PgConnection| {
            use crate::schema::visitor::dsl::*;

            let system_time = SystemTime::now();
            let dt: DateTime<UtcOffset> = system_time.into();

            diesel::insert_into(visitor)
                .values((
                    did.eq(user),
                    web.eq(service),
                    visited_at.eq(format!("{}", dt.format("%+"))),
                    feed.eq(requested_feed),
                ))
                .execute(conn)
                .expect("Error inserting visitor records");
            Ok(())
        })
        .await
        .map_err(|e| format!("Database interaction failed: {}", e))?;

    result
}

/// Retrieves usage statistics from the database.
pub async fn get_usage_stats(
    connection: ReadReplicaConn,
) -> Result<UsageStats, ValidationErrorMessageResponse> {
    use crate::schema::visitor::dsl as VisitorSchema;

    connection
        .0
        .get()
        .await
        .map_err(|_| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some("Failed to get database connection".to_string()),
        })?
        .interact(move |conn: &mut PgConnection| {
            let total_visits = VisitorSchema::visitor
                .count()
                .get_result::<i64>(conn)
                .unwrap_or(0);
            let unique_visitors = VisitorSchema::visitor
                .select(VisitorSchema::did)
                .distinct()
                .count()
                .get_result::<i64>(conn)
                .unwrap_or(0);

            let last_week = UtcOffset::now() - Duration::weeks(1);
            let weekly_unique_visitors = VisitorSchema::visitor
                .filter(VisitorSchema::visited_at.ge(format!("{}", last_week.format("%+"))))
                .select(VisitorSchema::did)
                .distinct()
                .count()
                .get_result::<i64>(conn)
                .unwrap_or(0);

            Ok(UsageStats {
                total_visits,
                unique_visitors,
                weekly_unique_visitors,
            })
        })
        .await
        .map_err(|e| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some(format!("Database interaction failed: {}", e)),
        })?
}

/// Retrieves visitor records from the database.
pub async fn get_visitors(
    connection: ReadReplicaConn,
) -> Result<Vec<Visitor>, ValidationErrorMessageResponse> {
    use crate::schema::visitor::dsl as VisitorSchema;

    connection
        .0
        .get()
        .await
        .map_err(|_| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some("Failed to get database connection".to_string()),
        })?
        .interact(move |conn: &mut PgConnection| {
            let results = VisitorSchema::visitor
                .order(VisitorSchema::visited_at.desc())
                .limit(100)
                .select(Visitor::as_select())
                .load(conn)
                .expect("Error loading visitor records");
            Ok(results)
        })
        .await
        .map_err(|e| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some(format!("Database interaction failed: {}", e)),
        })?
}

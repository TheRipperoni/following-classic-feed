use crate::db::*;
use crate::models::errors::error_code::ErrorCode;
use crate::models::errors::not_found_error_code::NotFoundErrorCode;
use crate::models::errors::path_unknown_error_message_response::PathUnknownErrorMessageResponse;
use crate::models::errors::validation_error_message_response::ValidationErrorMessageResponse;
use crate::models::*;
use crate::{ReadReplicaConn, WriteDbConn};
use diesel::pg::PgConnection;
use diesel::prelude::*;

#[tracing::instrument(skip(connection))]
pub async fn update_cursor(
    service: String,
    sequence: i64,
    connection: WriteDbConn,
) -> Result<(), String> {
    let new_update_state = CursorUpdateState {
        service,
        cursor: sequence,
    };

    let result = connection
        .0
        .get()
        .await
        .map_err(|e| format!("Failed to get database connection: {}", e))?
        .interact(move |conn: &mut PgConnection| {
            update_cursor_db(new_update_state, conn);
            Ok(())
        })
        .await
        .map_err(|e| format!("Database interaction failed: {}", e))?;

    result
}

pub async fn get_cursor(
    service_: String,
    connection: ReadReplicaConn,
) -> Result<SubState, PathUnknownErrorMessageResponse> {
    use crate::schema::sub_state::dsl::*;

    let result = connection
        .0
        .get()
        .await
        .map_err(|_| PathUnknownErrorMessageResponse {
            code: Some(NotFoundErrorCode::NotFoundError),
            message: Some("Failed to get database connection.".into()),
        })?
        .interact(move |conn: &mut PgConnection| {
            let mut result = sub_state
                .filter(service.eq(service_))
                .order(cursor.desc())
                .limit(1)
                .select(SubState::as_select())
                .load(conn)
                .expect("Error loading cursor records");

            if let Some(cursor_) = result.pop() {
                Ok(cursor_)
            } else {
                let not_found_error = PathUnknownErrorMessageResponse {
                    code: Some(NotFoundErrorCode::NotFoundError),
                    message: Some("Not found.".into()),
                };
                Err(not_found_error)
            }
        })
        .await
        .map_err(|e| PathUnknownErrorMessageResponse {
            code: Some(NotFoundErrorCode::NotFoundError),
            message: Some(format!("Database interaction failed: {}", e)),
        })??;

    Ok(result)
}

pub async fn get_janitor_config(
    connection: ReadReplicaConn,
) -> Result<JanitorConfig, ValidationErrorMessageResponse> {
    use crate::schema::janitor_config::dsl::*;

    connection
        .0
        .get()
        .await
        .map_err(|_| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some("Failed to get database connection".to_string()),
        })?
        .interact(move |conn: &mut PgConnection| {
            janitor_config
                .order(updated_at.desc())
                .limit(1)
                .select(JanitorConfig::as_select())
                .first(conn)
                .map_err(|e| ValidationErrorMessageResponse {
                    code: Some(ErrorCode::ValidationError),
                    message: Some(format!("Error loading janitor config: {}", e)),
                })
        })
        .await
        .map_err(|e| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some(format!("Database interaction failed: {}", e)),
        })?
}

pub async fn update_janitor_config(
    config: JanitorConfig,
    connection: WriteDbConn,
) -> Result<(), String> {
    use crate::schema::janitor_config::dsl::*;

    connection
        .0
        .get()
        .await
        .map_err(|e| format!("Failed to get database connection: {}", e))?
        .interact(move |conn: &mut PgConnection| {
            diesel::update(janitor_config.filter(id.eq(config.id)))
                .set((
                    cron_schedule.eq(config.cron_schedule),
                    retention_days.eq(config.retention_days),
                    updated_at.eq(diesel::dsl::now),
                ))
                .execute(conn)
                .map_err(|e| format!("Error updating janitor config: {}", e))
        })
        .await
        .map_err(|e| format!("Database interaction failed: {}", e))??;

    Ok(())
}

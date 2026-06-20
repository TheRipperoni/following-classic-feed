use crate::schema::backfill_job;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = backfill_job)]
pub struct BackfillJob {
    pub id: i32,
    pub did: String,
    pub state: String,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = backfill_job)]
pub struct NewBackfillJob {
    pub did: String,
}

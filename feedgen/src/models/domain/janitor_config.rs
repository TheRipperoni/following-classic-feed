use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(
    Queryable, Selectable, Clone, Debug, PartialEq, Default, Serialize, Deserialize, AsChangeset,
)]
#[diesel(table_name = crate::schema::janitor_config)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct JanitorConfig {
    #[serde(rename = "id")]
    pub id: i32,
    #[serde(rename = "cron_schedule")]
    pub cron_schedule: String,
    #[serde(rename = "retention_days")]
    pub retention_days: i32,
    #[serde(rename = "updated_at")]
    pub updated_at: NaiveDateTime,
}

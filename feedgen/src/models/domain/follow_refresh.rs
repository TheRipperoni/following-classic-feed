use crate::schema::follow_refresh;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = follow_refresh)]
pub struct FollowRefresh {
    pub did: String,
    pub refreshed_at: NaiveDateTime,
}

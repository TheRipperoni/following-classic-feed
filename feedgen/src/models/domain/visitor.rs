use crate::schema::visitor;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Queryable, Selectable, Insertable, Clone, Debug, PartialEq, Default, Serialize, Deserialize,
)]
#[diesel(table_name = visitor)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Visitor {
    pub id: i32,
    pub did: String,
    pub web: String,
    pub visited_at: String,
    pub feed: Option<String>,
}

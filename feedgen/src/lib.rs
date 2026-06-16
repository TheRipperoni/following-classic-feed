#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;
extern crate lexicon;

use deadpool_diesel::postgres::{Object, Pool};

#[derive(Clone)]
pub struct WriteDbConn(pub Pool);
#[derive(Clone)]
pub struct ReadReplicaConn(pub Pool);

pub type DbObject = Object;

pub mod agent;
pub mod apis;
pub mod auth;
pub mod db;
pub mod handlers;
pub mod metrics;
pub mod models;
pub use db_schema::schema;
pub mod state;

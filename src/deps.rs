use crate::persistence::Persistence;
use std::env::var;

pub fn get_database_url() -> String {
    var("DATABASE_URL").unwrap()
}

pub fn get_persistence_manager(database_url: Option<String>) -> Persistence {
    let database_url = database_url.unwrap_or_else(|| get_database_url());
    Persistence { database_url }
}

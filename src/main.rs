mod backend;
mod database;
mod frontend;

use crate::database::Database;

use clap::Parser;
use std::path::PathBuf;

fn default_database_location() -> PathBuf {
    dirs::home_dir().unwrap().join("benchmarker.db")
}

fn main() {
    let _db = Database::new(default_database_location()); // TODO use default database location or provided in argument (config?)
}

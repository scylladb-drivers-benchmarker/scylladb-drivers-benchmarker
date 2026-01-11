use std::path::Path;

use scylladb_drivers_benchmarker::database::Database;

pub fn open_clean_db(path: &Path) -> Database {
    let db = Database::new(path).unwrap();
    db.drop_all_data().unwrap();
    return db;
}

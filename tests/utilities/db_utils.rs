use std::path::Path;

use scylladb_drivers_benchmarker::database::Database;

pub fn open_clean_db(path: &Path) -> Database {
    let db = Database::new(path).expect(&format!("Couldn't open at path: {}", path.display()));
    db.drop_all_data().unwrap();
    db
}

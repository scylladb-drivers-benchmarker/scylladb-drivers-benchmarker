use std::path::Path;

use scylladb_drivers_benchmarker::database::Database;
use tempfile::NamedTempFile;


pub struct LocalDatabase {
    pub db: Database,
    pub file: NamedTempFile,
}

impl LocalDatabase {
    pub fn new() -> LocalDatabase {
        let file = NamedTempFile::new().expect("Cannot create a local db");
        let path = file.path().to_path_buf();
        LocalDatabase{
            db: Database::new(path).unwrap(), 
            file
        }
    }
}

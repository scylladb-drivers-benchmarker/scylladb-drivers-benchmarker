use sqlite::Connection;

use crate::PathBuf;
pub struct Database {
    connection: Connection,
}

pub struct BenchmarkParams {
    commit_hash: String,
    benchmark_type: String,
    benchmark_argument: u64,
    measurement_method: String,
}

pub struct BenchmarkResult {
    data_json: Option<String>,
}

impl Database {
    pub fn new(path: PathBuf) -> Database {
        let db = Database {
            connection: Connection::open(path).expect("couldn't access database"),
        };

        db.connection
            .execute(
                "
            CREATE TABLE IF NOT EXISTS Benchmarks (
                id INTEGER PRIMARY KEY,
                commit_hash TEXT NOT NULL,
                benchmark_type TEXT NOT NULL,
                benchmark_argument TEXT NOT NULL,
                measurement_method TEXT NOT NULL,
                data_json TEXT,
            );
            ",
            )
            .unwrap();

        db
    }

    pub fn insert_data(&self, params: BenchmarkParams, result: BenchmarkResult) {
        todo!();
    }

    pub fn get_data(&self, params: BenchmarkParams) {
        todo!();
    }

    pub fn data_exists(&self, params: BenchmarkParams) -> bool {
        todo!();
    }
}

use sqlite::Connection;
use sqlite::State;

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

impl BenchmarkResult {
    pub fn new(data_json: Option<String>) -> BenchmarkResult {
        BenchmarkResult { data_json }
    }

    pub fn isTimeout(&self) -> bool {
        return self.data_json.is_none();
    }
}

impl Database {
    pub fn new(path: PathBuf) -> Result<Database, sqlite::Error> {
        let db = Database {
            connection: Connection::open(path)?,
        };

        db.connection.execute(
            "
            CREATE TABLE IF NOT EXISTS Benchmarks (
                commit_hash TEXT NOT NULL,
                benchmark_type TEXT NOT NULL,
                benchmark_argument INTEGER NOT NULL,
                measurement_method TEXT NOT NULL,
                data_json TEXT,
                UNIQUE(commit_hash, benchmark_type, benchmark_argument, measurement_method),
            );
            ",
        )?;

        Ok(db)
    }

    pub fn insert_data(
        &self,
        params: BenchmarkParams,
        result: BenchmarkResult,
    ) -> Result<(), sqlite::Error> {
        let mut stmt = self.connection.prepare(
            "
                INSERT INTO Benchmarks
                (commit_hash, benchmark_type, benchmark_argument, measurement_method, data_json)
                VALUES (?, ?, ?, ?, ?);
                ",
        )?;

        stmt.bind((1, params.commit_hash.as_str()))?;
        stmt.bind((2, params.benchmark_type.as_str()))?;
        stmt.bind((3, params.benchmark_argument.to_string().as_str()))?;
        stmt.bind((4, params.measurement_method.as_str()))?;

        if !result.isTimeout() {
            stmt.bind((5, result.data_json.unwrap().as_str()))?;
        }

        stmt.next()?;
        Ok(())
    }

    pub fn get_data(
        &self,
        params: BenchmarkParams,
    ) -> Result<Option<BenchmarkResult>, sqlite::Error> {
        let mut stmt = self.connection.prepare(
            "
                SELECT data_json from  Benchmarks
                WHERE commit_hash = ? 
                    and benchmark_type = ?
                    and benchmark_argument = ?
                    and measurement_method = ?;
                ",
        )?;

        stmt.bind((1, params.commit_hash.as_str()))?;
        stmt.bind((2, params.benchmark_type.as_str()))?;
        stmt.bind((3, params.benchmark_argument.to_string().as_str()))?;
        stmt.bind((4, params.measurement_method.as_str()))?;

        stmt.next()?;
        let data_json: String = stmt.read(0)?;

        Ok(Some(BenchmarkResult {
            data_json: Some(data_json),
        }))
    }

    pub fn data_exists(&self, params: BenchmarkParams) -> Result<bool, sqlite::Error> {
        Ok(self.get_data(params)?.is_some())
    }
}

mod tests {
    #[test]
    fn test() {}
}

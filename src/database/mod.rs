use sqlite::Connection;

use crate::{PathBuf, commit_hash::CommitHash};
use sqlite::State;
pub struct Database {
    connection: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkParams {
    commit_hash: CommitHash,
    benchmark_type: String,
    benchmark_argument: u64,
    measurement_method: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkResult {
    data_json: Option<String>,
}

impl BenchmarkParams {
    pub fn new(
        commit_hash: CommitHash,
        benchmark_type: String,
        benchmark_argument: u64,
        measurement_method: String,
    ) -> BenchmarkParams {
        BenchmarkParams {
            commit_hash,
            benchmark_type,
            benchmark_argument,
            measurement_method,
        }
    }
}

impl BenchmarkResult {
    pub fn new(data_json: Option<String>) -> BenchmarkResult {
        BenchmarkResult { data_json }
    }

    pub fn isTimeout(&self) -> bool {
        self.data_json.is_none()
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
                UNIQUE(commit_hash, benchmark_type, benchmark_argument, measurement_method)
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

        match stmt.next()? {
            State::Row => {
                let data: Option<String> = stmt.read(0)?;
                Ok(Some(BenchmarkResult::new(data)))
            }
            State::Done => Ok(None),
        }
    }

    pub fn data_exists(&self, params: BenchmarkParams) -> Result<bool, sqlite::Error> {
        Ok(self.get_data(params)?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use crate::database::*;
    use tempfile::NamedTempFile;

    fn get_db() -> (Database, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        (Database::new(path).unwrap(), file)
    }

    #[test]
    fn simple_insert_get() {
        let (db, _file) = get_db();
        let params = BenchmarkParams::new(
            CommitHash::new_unchecked("abc123".into()),
            "speed".into(),
            42,
            "cold".into(),
        );
        let result = BenchmarkResult::new(Some("result_result ".into()));

        db.insert_data(params.clone(), result).unwrap();

        let retrieved = db.get_data(params.clone()).unwrap().unwrap();

        assert!(db.data_exists(params).unwrap());
        assert!(!retrieved.isTimeout());
        assert_eq!(retrieved.data_json.unwrap(), "result_result ");
    }

    #[test]
    fn insert_get_empty_and_timeout() {
        let (db, _file) = get_db();

        let params1 = BenchmarkParams::new(
            CommitHash::new_unchecked("abc123".into()),
            "speed".into(),
            42,
            "cold".into(),
        );
        let result_empty = BenchmarkResult::new(Some("".into()));

        let params2 = BenchmarkParams::new(
            CommitHash::new_unchecked("abc124".into()),
            "speed".into(),
            42,
            "cold".into(),
        );
        let result_timeout = BenchmarkResult::new(None);

        db.insert_data(params1.clone(), result_empty).unwrap();
        db.insert_data(params2.clone(), result_timeout).unwrap();

        let retrieved_empty = db.get_data(params1).unwrap().unwrap();
        let retrieved_timeout = db.get_data(params2).unwrap().unwrap();

        assert!(retrieved_timeout.isTimeout());
        assert_eq!(retrieved_empty.data_json.unwrap(), "");
    }

    #[test]
    fn get_doesnt_exist() {
        let (db, _file) = get_db();

        let params = BenchmarkParams::new(
            CommitHash::new_unchecked("missing".into()),
            "none".into(),
            123,
            "nope".into(),
        );

        let result = db.get_data(params).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn double_insert() {
        let (db, _file) = get_db();
        let params = BenchmarkParams::new(
            CommitHash::new_unchecked("abc123".into()),
            "speed".into(),
            42,
            "cold".into(),
        );
        let result1 = BenchmarkResult::new(Some("result_result ".into()));
        let result2 = BenchmarkResult::new(Some("result_result_result ".into()));

        db.insert_data(params.clone(), result1.clone()).unwrap();

        assert!(db.insert_data(params.clone(), result1).is_err());
        assert!(db.insert_data(params, result2).is_err());
    }
}

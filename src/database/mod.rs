use std::path::PathBuf;

use sqlite::Connection;
use sqlite::State;

use crate::utilities::BenchmarkParams;
use crate::utilities::BenchmarkRecord;

#[cfg(test)]
use crate::CommitHash;

pub struct Database {
    connection: Connection,
}

#[justerror::Error(desc = "internal database error")]
pub enum DatabaseError {
    InternalError(
        #[source]
        #[from]
        sqlite::Error,
    ),
}

impl Database {
    pub fn new(path: PathBuf) -> Result<Database, DatabaseError> {
        let db = Database {
            connection: Connection::open(path)?,
        };

        db.connection.execute(
            "
            CREATE TABLE IF NOT EXISTS Benchmarks (
                commit_hash TEXT NOT NULL,
                benchmark_name TEXT NOT NULL,
                benchmark_point INTEGER NOT NULL,
                measurement_method TEXT NOT NULL,
                data_json TEXT,
                UNIQUE(commit_hash, benchmark_name, benchmark_point, measurement_method)
            );
            ",
        )?;

        Ok(db)
    }

    pub fn insert_data(
        &self,
        params: BenchmarkParams,
        result: BenchmarkRecord,
    ) -> Result<(), DatabaseError> {
        let mut stmt = self.connection.prepare(
            "
                INSERT INTO Benchmarks
                (commit_hash, benchmark_name, benchmark_point, measurement_method, data_json)
                VALUES (?, ?, ?, ?, ?);
                ",
        )?;

        stmt.bind((1, params.commit_hash.as_str()))?;
        stmt.bind((2, params.benchmark_name.as_str()))?;
        stmt.bind((3, params.benchmark_point.to_string().as_str()))?;
        stmt.bind((4, params.measurement_method.as_str()))?;

        if let Some(data_json) = result.data_json {
            stmt.bind((5, data_json.as_str()))?;
        }

        stmt.next()?;
        Ok(())
    }

    pub fn get_data(
        &self,
        params: BenchmarkParams,
    ) -> Result<Option<BenchmarkRecord>, DatabaseError> {
        let mut stmt = self.connection.prepare(
            "
                SELECT data_json from  Benchmarks
                WHERE commit_hash = ? 
                    and benchmark_name = ?
                    and benchmark_point = ?
                    and measurement_method = ?;
                ",
        )?;

        stmt.bind((1, params.commit_hash.as_str()))?;
        stmt.bind((2, params.benchmark_name.as_str()))?;
        stmt.bind((3, params.benchmark_point.to_string().as_str()))?;
        stmt.bind((4, params.measurement_method.as_str()))?;

        match stmt.next()? {
            State::Row => {
                let data: Option<String> = stmt.read(0)?;
                Ok(Some(BenchmarkRecord::new(data)))
            }
            State::Done => Ok(None),
        }
    }

    pub fn data_exists(&self, params: BenchmarkParams) -> Result<bool, DatabaseError> {
        Ok(self.get_data(params)?.is_some())
    }

    #[cfg(test)]
    pub fn get_all_data(&self) -> Result<Vec<(BenchmarkParams, BenchmarkRecord)>, DatabaseError> {
        let mut stmt = self.connection.prepare(
            "SELECT commit_hash, benchmark_name, benchmark_point, measurement_method, data_json FROM Benchmarks;",
        )?;

        let mut results = Vec::new();

        while let State::Row = stmt.next()? {
            let commit_hash_str: String = stmt.read(0)?;
            let benchmark_name: String = stmt.read(1)?;
            let benchmark_point: u64 = stmt.read::<i64, usize>(2)?.try_into().unwrap();
            let measurement_method: String = stmt.read(3)?;
            let data_json: Option<String> = stmt.read(4)?;

            let params = BenchmarkParams::new(
                CommitHash::new_unchecked(commit_hash_str),
                benchmark_name,
                benchmark_point,
                measurement_method,
            );
            let record = BenchmarkRecord::new(data_json);

            results.push((params, record));
        }

        Ok(results)
    }

    #[cfg(test)]
    pub unsafe fn drop_table(&self) -> Result<(), DatabaseError> {
        self.connection.execute("DELETE FROM Benchmarks;")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{commit_hash::CommitHash, database::*};
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
        let result = BenchmarkRecord::new(Some("result_result ".into()));

        db.insert_data(params.clone(), result).unwrap();

        let retrieved = db.get_data(params.clone()).unwrap().unwrap();

        assert!(db.data_exists(params).unwrap());
        assert!(!retrieved.is_timeout());
        assert_eq!(retrieved.data_json.unwrap(), "result_result ");
    }

    #[test]
    fn insert_get_empty_imeout_all_clear() {
        let (db, _file) = get_db();

        let params1 = BenchmarkParams::new(
            CommitHash::new_unchecked("abc123".into()),
            "speed".into(),
            42,
            "cold".into(),
        );
        let result_empty = BenchmarkRecord::new(Some("".into()));

        let params2 = BenchmarkParams::new(
            CommitHash::new_unchecked("abc124".into()),
            "speed".into(),
            42,
            "cold".into(),
        );
        let result_timeout = BenchmarkRecord::new(None);

        db.insert_data(params1.clone(), result_empty.clone())
            .unwrap();
        db.insert_data(params2.clone(), result_timeout.clone())
            .unwrap();

        let retrieved_empty = db.get_data(params1.clone()).unwrap().unwrap();
        let retrieved_timeout = db.get_data(params2.clone()).unwrap().unwrap();

        assert!(retrieved_timeout.is_timeout());
        assert_eq!(retrieved_empty.data_json.unwrap(), "");

        let data = db.get_all_data().unwrap();

        assert!(data.len() == 2);
        assert!(
            (data[0] == (params1.clone(), result_empty.clone())
                && data[1] == (params2.clone(), result_timeout.clone()))
                || (data[1] == (params1.clone(), result_empty.clone())
                    && data[0] == (params2.clone(), result_timeout.clone()))
        );

        unsafe {
            db.drop_table().unwrap();
        }

        let data = db.get_all_data().unwrap();
        assert!(data.is_empty());
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
        let result1 = BenchmarkRecord::new(Some("result_result ".into()));
        let result2 = BenchmarkRecord::new(Some("result_result_result ".into()));

        db.insert_data(params.clone(), result1.clone()).unwrap();

        assert!(db.insert_data(params.clone(), result1).is_err());
        assert!(db.insert_data(params, result2).is_err());
    }
}

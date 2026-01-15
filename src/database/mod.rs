pub mod utilities;

#[cfg(test)]
mod tests;

use std::path::Path;

use sqlite::{Connection, State, Statement};

use crate::CommitHash;
use crate::database::utilities::*;
use std::path::PathBuf;

pub struct Database {
    connection: Connection,
}

#[justerror::Error(desc = "internal database error")]
pub enum DatabaseError {
    InternalError(#[from] sqlite::Error),

    JsonError(#[from] serde_json::Error),

    #[error(desc = "Multiple results for same params in database")]
    MultipleResults,

    #[error(
        desc = "Provided database contains table Benchmarks with wrong scheme.\nExpected: {0}.\nFound: {1}."
    )]
    WrongTableExists(String, String),

    #[error(desc = "Failed to remove file at path: {path}")]
    FileRemovalError {
        source: std::io::Error,
        path: PathBuf,
    },
}

impl Database {
    fn validate_schema(&self) -> Result<(), DatabaseError> {
        let mut stmt = self.connection.prepare("PRAGMA table_info(Benchmarks);")?;

        let expected = vec![
            ("commit_hash".to_owned(), "TEXT".to_owned(), true),
            ("benchmark_name".to_owned(), "TEXT".to_owned(), true),
            ("benchmark_point".to_owned(), "INTEGER".to_owned(), true),
            ("measurement_method".to_owned(), "TEXT".to_owned(), true),
            ("data_json".to_owned(), "TEXT".to_owned(), true),
        ];

        let mut found = Vec::new();

        while let Ok(sqlite::State::Row) = stmt.next() {
            let name: String = stmt.read("name")?;
            let ty: String = stmt.read("type")?;
            let notnull: i64 = stmt.read("notnull")?;
            found.push((name, ty.to_uppercase(), notnull == 1));
        }
        if found != expected {
            return Err(DatabaseError::WrongTableExists(
                format!("{:?}", expected),
                format!("{:?}", found),
            ));
        }
        Ok(())
    }

    fn bind_params<'stmt>(
        stmt: &mut Statement<'stmt>,
        params: BenchmarkParams,
    ) -> Result<(), DatabaseError> {
        stmt.bind((1, params.commit_hash.as_str()))?;
        stmt.bind((2, params.benchmark_name.as_str()))?;
        stmt.bind((3, params.benchmark_point as i64))?;
        stmt.bind((4, params.measurement_method.as_str()))?;
        Ok(())
    }

    /// Returns WHERE clause:
    /// "WHERE commit_hash IN (...) AND benchmark_name IN (...) ..."
    /// or empty string if no filters.
    fn data_filtration(&self, filters: &BenchmarkFilters) -> String {
        // Helper function to build in clause for one column.
        fn build_in_clause<T: ToString>(column_name: &str, values: &[T]) -> Option<String> {
            if values.is_empty() {
                return None;
            }

            let formatted_values: Vec<String> = values
                .iter()
                .map(|v| {
                    let s = v.to_string();
                    // If numeric, keep as is; if string, wrap in quotes and escape
                    if s.parse::<i64>().is_ok() {
                        s
                    } else {
                        format!("'{}'", s.replace('\'', "''"))
                    }
                })
                .collect();

            Some(format!(
                "{} IN ({})",
                column_name,
                formatted_values.join(", ")
            ))
        }

        let clauses: Vec<String> = [
            build_in_clause("commit_hash", &filters.commit_hashes),
            build_in_clause("benchmark_name", &filters.benchmark_names),
            build_in_clause("benchmark_point", &filters.benchmark_points),
            build_in_clause("measurement_method", &filters.measurement_methods),
        ]
        .into_iter()
        .flatten()
        .collect();

        if clauses.is_empty() {
            "".to_owned()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        }
    }

    pub fn new(path: &Path) -> Result<Database, DatabaseError> {
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
                data_json TEXT NOT NULL,
                UNIQUE(commit_hash, benchmark_name, benchmark_point, measurement_method)
            );
            ",
        )?;

        // CREATE TABLE IF NOT EXISTS just checks if table "Benchmarks" exists.
        // It may lead to strange errors, if table Benchmarks was already in database
        // with diffrent column names/types or wrong constraint.
        // Validation of constraints is hard and probably not worth it.
        // Just check column names and types.
        db.validate_schema()?;

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

        Database::bind_params(&mut stmt, params)?;
        stmt.bind::<(usize, &str)>((5, &serde_json::to_string(&result)?))?;

        stmt.next()?;

        Ok(())
    }

    pub fn get_data(
        &self,
        filters: &BenchmarkFilters,
    ) -> Result<Vec<(BenchmarkParams, BenchmarkRecord)>, DatabaseError> {
        let mut stmt = self
            .connection
            .prepare("SELECT * FROM Benchmarks ".to_owned() + &self.data_filtration(filters))?;

        let mut results = Vec::new();

        while let State::Row = stmt.next()? {
            let commit_hash_str: String = stmt.read(0)?;
            let benchmark_name: String = stmt.read(1)?;
            let benchmark_point: u64 = stmt.read::<i64, usize>(2)? as u64;
            let measurement_method: String = stmt.read(3)?;
            let result: BenchmarkRecord = serde_json::from_str(&stmt.read::<String, usize>(4)?)?;

            let params = BenchmarkParams::new(
                CommitHash::new_unchecked(commit_hash_str),
                benchmark_name,
                benchmark_point,
                measurement_method,
            );

            results.push((params, result));
        }

        Ok(results)
    }

    pub fn get_all_data(&self) -> Result<Vec<(BenchmarkParams, BenchmarkRecord)>, DatabaseError> {
        self.get_data(&BenchmarkFilters::all())
    }

    /// Drops data matching filters from database.
    /// If any of the dropped records is FilePath, also removes the file.
    pub fn drop_data(&self, filters: &BenchmarkFilters) -> Result<(), DatabaseError> {
        let to_be_dropped = self.get_data(filters)?;
        for (_, record) in to_be_dropped {
            if let BenchmarkRecord::FilePath(path) = record {
                std::fs::remove_file(&path)
                    .map_err(|e| DatabaseError::FileRemovalError { source: e, path })?;
            }
        }

        self.connection
            .prepare("DELETE FROM Benchmarks ".to_owned() + &self.data_filtration(filters))?
            .next()?;

        Ok(())
    }

    /// Drops all data from database.
    /// If any of the dropped records is FilePath, also removes the file.
    pub fn drop_all_data(&self) -> Result<(), DatabaseError> {
        self.drop_data(&BenchmarkFilters::all())
    }

    pub fn get_result(
        &self,
        params: BenchmarkParams,
    ) -> Result<Option<BenchmarkRecord>, DatabaseError> {
        let filters = BenchmarkFilters::filter_exact_param(&params);

        let results = self.get_data(&filters)?;

        let mut iter = results.into_iter();
        let Some(result) = iter.next() else {
            return Ok(None);
        };

        if iter.next().is_none() {
            Ok(Some(result.1))
        } else {
            Err(DatabaseError::MultipleResults)
        }
    }

    pub fn result_exists(&self, params: BenchmarkParams) -> Result<bool, DatabaseError> {
        Ok(self.get_result(params)?.is_some())
    }
}

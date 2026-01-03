pub mod utilities;
mod tests;

use std::path::Path;

use sqlite::Connection;
use sqlite::State;
use sqlite::Statement;

use crate::CommitHash;

use crate::database::utilities::{BenchmarkFilters, BenchmarkParams, BenchmarkRecord};

pub struct Database {
    connection: Connection,
}

#[justerror::Error(desc = "internal database error")]
pub enum DatabaseError {
    InternalError(#[from] sqlite::Error),

    JsonError(#[from] serde_json::Error),

    #[error(desc = "Multiple results for same params in database")]
    MultpleResults,
}

impl Database {
    fn bind_params<'stmt>(
        stmt: &mut Statement<'stmt>,
        params: BenchmarkParams,
    ) -> Result<(), DatabaseError> {
        stmt.bind((1, params.commit_hash.as_str()))?;
        stmt.bind((2, params.benchmark_name.as_str()))?;
        stmt.bind((3, params.benchmark_point.to_string().as_str()))?;
        stmt.bind((4, params.measurement_method.as_str()))?;
        Ok(())
    }

    /// Returns WHERE clause:
    /// "WHERE commit_hash IN (...) AND benchmark_name IN (...) ..."
    /// or empty string if no filters.
    fn data_filtration(&self, filters: &BenchmarkFilters) -> String {
        // Helper function to build in cluase for one column.
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

    pub fn drop_data(&self, filters: &BenchmarkFilters) -> Result<(), DatabaseError> {
        self.connection
            .prepare("DELETE FROM Benchmarks ".to_owned() + &self.data_filtration(filters))?
            .next()?;

        Ok(())
    }

    pub fn drop_all_data(&self) -> Result<(), DatabaseError> {
        self.drop_data(&BenchmarkFilters::all())
    }

    pub fn get_result(
        &self,
        params: BenchmarkParams,
    ) -> Result<Option<BenchmarkRecord>, DatabaseError> {
        let filters = BenchmarkFilters::filter_exact_param(&params);

        let results = self.get_data(&filters)?;

        match results.len() {
            0 => Ok(None),
            1 => Ok(Some(results.into_iter().next().unwrap().1)),
            _ => Err(DatabaseError::MultpleResults),
        }
    }

    pub fn result_exists(&self, params: BenchmarkParams) -> Result<bool, DatabaseError> {
        Ok(self.get_result(params)?.is_some())
    }
}

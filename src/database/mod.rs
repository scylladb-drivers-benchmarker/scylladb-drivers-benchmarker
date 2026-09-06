pub mod utilities;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use sqlite::{Connection, State, Statement};

use crate::CommitHash;
use crate::database::utilities::{BenchmarkFilters, BenchmarkParams, BenchmarkRecord, Provenance};

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
            ("driver_name".to_owned(), "TEXT".to_owned(), true),
            ("benchmark_point".to_owned(), "INTEGER".to_owned(), true),
            ("measurement_method".to_owned(), "TEXT".to_owned(), true),
            ("api".to_owned(), "TEXT".to_owned(), true),
            ("benchmarks_commit".to_owned(), "TEXT".to_owned(), true),
            ("timestamp".to_owned(), "TEXT".to_owned(), true),
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
                format!("{expected:?}"),
                format!("{found:?}"),
            ));
        }
        Ok(())
    }

    fn bind_params(stmt: &mut Statement<'_>, params: BenchmarkParams) -> Result<(), DatabaseError> {
        stmt.bind((1, params.commit_hash.as_str()))?;
        stmt.bind((2, params.benchmark_name.as_str()))?;
        stmt.bind((3, params.driver_name.as_str()))?;
        stmt.bind((4, params.benchmark_point as i64))?;
        stmt.bind((5, params.measurement_method.as_str()))?;
        Ok(())
    }

    /// Returns a WHERE clause with `?` placeholders and the values to bind,
    /// or an empty clause if there are no filters.
    fn data_filtration(filters: &BenchmarkFilters) -> (String, Vec<sqlite::Value>) {
        let mut clauses: Vec<String> = Vec::new();
        let mut values: Vec<sqlite::Value> = Vec::new();

        fn add_in_clause(
            clauses: &mut Vec<String>,
            values: &mut Vec<sqlite::Value>,
            column_name: &str,
            filter_values: impl ExactSizeIterator<Item = sqlite::Value>,
        ) {
            if filter_values.len() == 0 {
                return;
            }
            let placeholders = vec!["?"; filter_values.len()].join(", ");
            clauses.push(format!("{column_name} IN ({placeholders})"));
            values.extend(filter_values);
        }

        let strings = |v: &[String]| {
            v.iter()
                .map(|s| sqlite::Value::String(s.clone()))
                .collect::<Vec<_>>()
                .into_iter()
        };
        add_in_clause(&mut clauses, &mut values, "commit_hash", strings(&filters.commit_hashes));
        add_in_clause(&mut clauses, &mut values, "benchmark_name", strings(&filters.benchmark_names));
        add_in_clause(&mut clauses, &mut values, "driver_name", strings(&filters.driver_names));
        add_in_clause(
            &mut clauses,
            &mut values,
            "benchmark_point",
            filters
                .benchmark_points
                .iter()
                .map(|p| sqlite::Value::Integer(*p as i64))
                .collect::<Vec<_>>()
                .into_iter(),
        );
        add_in_clause(&mut clauses, &mut values, "measurement_method", strings(&filters.measurement_methods));

        if clauses.is_empty() {
            (String::new(), values)
        } else {
            (format!("WHERE {}", clauses.join(" AND ")), values)
        }
    }

    fn prepare_filtered(
        &self,
        query_prefix: &str,
        query_suffix: &str,
        filters: &BenchmarkFilters,
    ) -> Result<Statement<'_>, DatabaseError> {
        let (clause, values) = Self::data_filtration(filters);
        let mut stmt = self
            .connection
            .prepare(format!("{query_prefix} {clause} {query_suffix}"))?;
        for (i, value) in values.into_iter().enumerate() {
            stmt.bind((i + 1, value))?;
        }
        Ok(stmt)
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
                driver_name TEXT NOT NULL,
                benchmark_point INTEGER NOT NULL,
                measurement_method TEXT NOT NULL,
                api TEXT NOT NULL,
                benchmarks_commit TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                data_json TEXT NOT NULL,
                UNIQUE(commit_hash, benchmark_name, driver_name, benchmark_point, measurement_method)
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
        provenance: Provenance,
        result: BenchmarkRecord,
    ) -> Result<(), DatabaseError> {
        let mut stmt = self.connection.prepare(
            "
                INSERT INTO Benchmarks
                (commit_hash, benchmark_name, driver_name, benchmark_point, measurement_method, api, benchmarks_commit, timestamp, data_json)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?);
                ",
        )?;

        Database::bind_params(&mut stmt, params)?;
        stmt.bind((6, provenance.api.as_str()))?;
        stmt.bind((7, provenance.benchmarks_commit.as_str()))?;
        stmt.bind((8, utilities::current_timestamp().as_str()))?;
        stmt.bind::<(usize, &str)>((9, &serde_json::to_string(&result)?))?;

        stmt.next()?;

        Ok(())
    }

    pub fn get_data(
        &self,
        filters: &BenchmarkFilters,
    ) -> Result<Vec<(BenchmarkParams, BenchmarkRecord)>, DatabaseError> {
        let mut stmt = self.prepare_filtered(
            "SELECT commit_hash, benchmark_name, driver_name, benchmark_point, measurement_method, data_json FROM Benchmarks",
            "",
            filters,
        )?;

        let mut results = Vec::new();

        while let State::Row = stmt.next()? {
            let commit_hash_str: String = stmt.read(0)?;
            let benchmark_name: String = stmt.read(1)?;
            let driver_name: String = stmt.read(2)?;
            let benchmark_point: u64 = stmt.read::<i64, usize>(3)? as u64;
            let measurement_method: String = stmt.read(4)?;
            let result: BenchmarkRecord = serde_json::from_str(&stmt.read::<String, usize>(5)?)?;

            let params = BenchmarkParams::new(
                CommitHash::new_unchecked(commit_hash_str),
                benchmark_name,
                driver_name,
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
    /// If any of the dropped records is `FilePath`, also removes the file.
    pub fn drop_data(&self, filters: &BenchmarkFilters) -> Result<(), DatabaseError> {
        let to_be_dropped = self.get_data(filters)?;
        for (_, record) in to_be_dropped {
            if let BenchmarkRecord::FilePath(path) = record {
                std::fs::remove_file(&path)
                    .map_err(|e| DatabaseError::FileRemovalError { source: e, path })?;
            }
        }

        self.prepare_filtered("DELETE FROM Benchmarks", "", filters)?
            .next()?;

        Ok(())
    }

    /// Drops all data from database.
    /// If any of the dropped records is `FilePath`, also removes the file.
    pub fn drop_all_data(&self) -> Result<(), DatabaseError> {
        self.drop_data(&BenchmarkFilters::all())
    }

    /// Returns distinct backend names for a given commit hash, benchmark name,
    /// and measurement method, sorted alphabetically.
    pub fn get_driver_names(
        &self,
        commit_hash: &CommitHash,
        benchmark_name: &str,
        measurement_method: &str,
    ) -> Result<Vec<String>, DatabaseError> {
        let filters = BenchmarkFilters {
            commit_hashes: vec![commit_hash.as_str().to_owned()],
            benchmark_names: vec![benchmark_name.to_owned()],
            driver_names: vec![],
            benchmark_points: vec![],
            measurement_methods: vec![measurement_method.to_owned()],
        };

        let mut stmt = self.prepare_filtered(
            "SELECT DISTINCT driver_name FROM Benchmarks",
            "ORDER BY driver_name",
            &filters,
        )?;
        let mut driver_names = Vec::new();
        while let State::Row = stmt.next()? {
            driver_names.push(stmt.read::<String, _>(0)?);
        }
        Ok(driver_names)
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

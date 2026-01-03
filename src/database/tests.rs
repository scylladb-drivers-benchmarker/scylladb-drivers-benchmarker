#[cfg(test)]
mod tests {
    use crate::{commit_hash::CommitHash, database::*};
    use tempfile::NamedTempFile;

    fn get_db() -> (Database, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        (Database::new(&path).unwrap(), file)
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
        let result = BenchmarkRecord::Data("result_result ".into());

        db.insert_data(params.clone(), result).unwrap();

        let retrieved = db.get_result(params.clone()).unwrap().unwrap();

        assert!(db.result_exists(params).unwrap());
        assert!(retrieved == BenchmarkRecord::Data("result_result ".into()));
    }

    #[test]
    fn insert_get_empty_timeout_all_clear() {
        let (db, _file) = get_db();

        let params1 = BenchmarkParams::new(
            CommitHash::new_unchecked("abc123".into()),
            "speed".into(),
            42,
            "cold".into(),
        );
        let result_empty = BenchmarkRecord::Data("".into());

        let params2 = BenchmarkParams::new(
            CommitHash::new_unchecked("abc124".into()),
            "speed".into(),
            42,
            "cold".into(),
        );
        let result_timeout = BenchmarkRecord::Timeout;

        db.insert_data(params1.clone(), result_empty.clone())
            .unwrap();
        db.insert_data(params2.clone(), result_timeout.clone())
            .unwrap();

        let retrieved_empty = db.get_result(params1.clone()).unwrap().unwrap();
        let retrieved_timeout = db.get_result(params2.clone()).unwrap().unwrap();

        assert_eq!(retrieved_timeout, BenchmarkRecord::Timeout);
        assert_eq!(retrieved_empty, BenchmarkRecord::Data("".into()));

        let data = db.get_all_data().unwrap();

        assert!(data.len() == 2);
        assert!(
            (data[0] == (params1.clone(), result_empty.clone())
                && data[1] == (params2.clone(), result_timeout.clone()))
                || (data[1] == (params1.clone(), result_empty.clone())
                    && data[0] == (params2.clone(), result_timeout.clone()))
        );

        db.drop_all_data().unwrap();

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

        let result = db.get_result(params).unwrap();

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
        let result1 = BenchmarkRecord::Data("result_result ".into());
        let result2 = BenchmarkRecord::Data("result_result_result ".into());

        db.insert_data(params.clone(), result1.clone()).unwrap();

        assert!(db.insert_data(params.clone(), result1).is_err());
        assert!(db.insert_data(params, result2).is_err());
    }

    #[test]
    fn test_data_filtration() {
        let db = Database::new(Path::new(":memory:")).unwrap();

        let commit_hashes = vec!["a", "b"];
        let benchmark_names = vec!["x", "y"];
        let benchmark_points = vec![1, 2];
        let measurement_methods = vec!["cold", "hot"];

        // Add all data to database.
        // Test data_filtration (creating WHERE clouse).
        for &ch in &commit_hashes {
            for &bn in &benchmark_names {
                for &bp in &benchmark_points {
                    for &mm in &measurement_methods {
                        let filters = BenchmarkFilters {
                            commit_hashes: vec![ch.into()],
                            benchmark_names: vec![bn.into()],
                            benchmark_points: vec![bp],
                            measurement_methods: vec![mm.into()],
                        };

                        let sql = db.data_filtration(&filters);

                        let expected_sql = format!(
                            "WHERE commit_hash IN ('{}') AND benchmark_name IN ('{}') AND benchmark_point IN ({}) AND measurement_method IN ('{}')",
                            ch, bn, bp, mm
                        );

                        assert_eq!(
                            sql, expected_sql,
                            "Failed for combination: ch={}, bn={}, bp={}, mm={}",
                            ch, bn, bp, mm
                        );

                        let params = BenchmarkParams::new(
                            CommitHash::new_unchecked(ch.into()),
                            bn.into(),
                            bp,
                            mm.into(),
                        );
                        let record =
                            BenchmarkRecord::Data(format!("data_{}{}{}{}", ch, bn, bp, mm));
                        db.insert_data(params, record).unwrap();
                    }
                }
            }
        }

        // Test get_data functionality.
        let filters_list = vec![
            (
                BenchmarkFilters {
                    commit_hashes: vec!["a".into()],
                    benchmark_names: vec!["x".into()],
                    benchmark_points: vec![1],
                    measurement_methods: vec!["cold".into()],
                },
                1,
            ),
            (
                BenchmarkFilters {
                    commit_hashes: vec![],
                    benchmark_names: vec!["y".into()],
                    benchmark_points: vec![2],
                    measurement_methods: vec![
                        "'but_has_funny_chars'''''''".into(),
                        "hot".into(),
                        "'but_has_funny_chars'''''''".into(),
                    ],
                },
                2,
            ),
            (
                BenchmarkFilters {
                    commit_hashes: vec![],
                    benchmark_names: vec!["x".into()],
                    benchmark_points: vec![],
                    measurement_methods: vec!["cold".into()],
                },
                4,
            ),
        ];

        for (filters, expected_count) in filters_list {
            let results = db.get_data(&filters).unwrap();

            // Check number of rows
            assert_eq!(results.len(), expected_count);

            // Check that all rows match the filter
            for (params, _record) in results {
                if !filters.commit_hashes.is_empty() {
                    assert!(
                        filters
                            .commit_hashes
                            .contains(&String::from(params.commit_hash))
                    );
                }
                if !filters.benchmark_names.is_empty() {
                    assert!(filters.benchmark_names.contains(&params.benchmark_name));
                }
                if !filters.benchmark_points.is_empty() {
                    assert!(filters.benchmark_points.contains(&params.benchmark_point));
                }
                if !filters.measurement_methods.is_empty() {
                    assert!(
                        filters
                            .measurement_methods
                            .contains(&params.measurement_method)
                    );
                }
            }
        }

        // Test deleting from table.
        let filter_single = BenchmarkFilters {
            commit_hashes: vec!["a".into()],
            benchmark_names: vec!["x".into()],
            benchmark_points: vec![1],
            measurement_methods: vec!["cold".into(), "sasdsadasdads".into()],
        };

        let filter_multiple = BenchmarkFilters {
            commit_hashes: vec!["b".into()],
            benchmark_names: vec!["y".into()],
            benchmark_points: vec![],
            measurement_methods: vec!["hot".into()],
        };

        db.drop_data(&filter_single).unwrap();
        let remaining = db.get_all_data().unwrap();
        for (params, _record) in &remaining {
            assert!(
                !(String::from(params.commit_hash.clone()) == "a"
                    && params.benchmark_name == "x"
                    && params.benchmark_point == 1
                    && params.measurement_method == "cold"),
                "Row matching single filter was not deleted"
            );
        }

        db.drop_data(&filter_multiple).unwrap();
        let remaining = db.get_all_data().unwrap();
        for (params, _record) in &remaining {
            assert!(
                !(String::from(params.commit_hash.clone()) == "b"
                    && params.benchmark_name == "y"
                    && params.measurement_method == "hot"),
                "Row(s) matching multiple filter were not deleted"
            );
        }

        assert_eq!(
            remaining.len(),
            16 - 1 - 2,
            "Unexpected number of remaining rows"
        );
    }
}

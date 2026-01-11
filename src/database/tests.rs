use crate::{commit_hash::CommitHash, database::*};
use tempfile::NamedTempFile;

fn get_db() -> (Database, NamedTempFile) {
    let file = NamedTempFile::new().unwrap();
    (Database::new(file.path()).unwrap(), file)
}

fn example_param(hash: &str) -> BenchmarkParams {
    BenchmarkParams::new(
        CommitHash::new_unchecked(hash.to_owned()),
        "speed".to_owned(),
        42,
        "cold".to_owned(),
    )
}

// Helper function that inserts data to db, checks if they exists. Finally drops all and checks if db is empty.
fn insert_get(entries: Vec<(BenchmarkParams, BenchmarkRecord)>) {
    let (db, _file) = get_db();

    for (p, r) in &entries {
        db.insert_data(p.clone(), r.clone()).unwrap();
    }

    for (p, r) in &entries {
        assert_eq!(db.get_result(p.clone()).unwrap().unwrap(), *r);
    }

    let data = db.get_all_data().unwrap();
    for (p, r) in &entries {
        assert!(data.contains(&(p.clone(), r.clone())));
    }
    assert_eq!(data.len(), entries.len());

    db.drop_all_data().unwrap();
    assert!(db.get_all_data().unwrap().is_empty());
}

#[test]
fn simple_insert_get() {
    insert_get(vec![(
        example_param("test"),
        BenchmarkRecord::Data("Record!".to_owned()),
    )]);
}

#[test]
fn insert_get_empty() {
    insert_get(vec![
        (example_param("aa"), BenchmarkRecord::Data("".to_owned())),
        (example_param("bbb"), BenchmarkRecord::Timeout),
        (example_param("cccc"), BenchmarkRecord::FilePath("".into())),
    ]);
}

#[test]
fn doesnt_exist() {
    let (db, _file) = get_db();
    assert!(db.get_result(example_param("d")).unwrap().is_none());
}

#[test]
fn double_insert() {
    let (db, _file) = get_db();

    let result1 = BenchmarkRecord::Data("result_result ".to_owned());
    let result2 = BenchmarkRecord::Data("result_result_result ".to_owned());

    db.insert_data(example_param(""), result1.clone()).unwrap();

    assert!(db.insert_data(example_param(""), result1).is_err());
    assert!(db.insert_data(example_param(""), result2).is_err());
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
                    let params = BenchmarkParams::new(
                        CommitHash::new_unchecked(ch.into()),
                        bn.into(),
                        bp,
                        mm.into(),
                    );

                    let sql = db.data_filtration(&BenchmarkFilters::filter_exact_param(&params));
                    let expected_sql = format!(
                        "WHERE commit_hash IN ('{}') AND benchmark_name IN ('{}') AND benchmark_point IN ({}) AND measurement_method IN ('{}')",
                        ch, bn, bp, mm
                    );

                    assert_eq!(sql, expected_sql);

                    db.insert_data(params, BenchmarkRecord::Data("".to_owned()))
                        .unwrap();
                }
            }
        }
    }

    // For each vector element filter elements, and remove them.
    let filters_list = vec![
        // Filter, expected number of filtered elements (assuming all previously filtered are dropped)
        (
            BenchmarkFilters {
                commit_hashes: vec!["a".into()],
                benchmark_names: vec!["x".into()],
                benchmark_points: vec![1],
                measurement_methods: vec!["cold".into()],
            },
            1,  // Drops 1 element
            15, // 15 should be left
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
            2,  // Drops 2 elements
            13, // 13 should be left
        ),
        (
            BenchmarkFilters {
                commit_hashes: vec![],
                benchmark_names: vec!["x".into()],
                benchmark_points: vec![],
                measurement_methods: vec!["cold".into()],
            },
            3, // Drops 3 elements, because one was dropped previously
            10,
        ),
    ];

    for (filters, expected_filtered, expected_left) in filters_list {
        assert_eq!(db.get_data(&filters).unwrap().len(), expected_filtered);
        db.drop_data(&filters).unwrap();
        assert_eq!(expected_left, db.get_all_data().unwrap().len());
    }
}

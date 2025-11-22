use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BackendConfig {
    pub name: String,
    pub benchmark_name: String,
    pub build_command: String,
    pub run_command: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BackendConfigList {
    pub benchmarks: Vec<BackendConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_benchmark_config() {
        let config = BackendConfig {
            name: "scylladb-nodejs-rs-driver".to_string(),
            benchmark_name: "select".to_string(),
            build_command: "npm run build".to_string(),
            run_command: "node benchmark/logic/select.js scylladb-nodejs-rs-driver".to_string(),
        };

        let serialized: String = serde_yml::to_string(&config).unwrap();
        let expected: &str = "\
name: scylladb-nodejs-rs-driver
benchmark-name: select
build-command: npm run build
run-command: node benchmark/logic/select.js scylladb-nodejs-rs-driver
";
        assert_eq!(serialized, expected);
    }
}

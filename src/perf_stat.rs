use serde::Deserialize;
use serde::Deserializer;
use std::{str::FromStr};

// Valid json numbers contain dot, but perf-stat returns numbers with coma.
fn deserialize_coma_numbers<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.replace(',', ".")
        .parse::<f64>()
        .map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize, Clone)]
pub struct PerfEvent {
    #[serde(default)]
    pub event: String,

    #[serde(rename = "metric-value", deserialize_with = "deserialize_coma_numbers")]
    pub value: f64,

    #[serde(rename = "metric-unit")]
    pub unit: String,
}
#[derive(Debug)]
pub struct PerfStatData {
    pub events: Vec<PerfEvent>,
}

impl FromStr for PerfStatData {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, serde_json::Error> {
        let mut events = Vec::new();
        let deserializer= serde_json::Deserializer::from_str(s).into_iter::<PerfEvent>();
        for event in deserializer {
            events.push(event?);
        }
        Ok(PerfStatData { events })
    }
}

impl PerfStatData {
    pub fn filter_value(&self, event_name: &str) -> Option<PerfEvent> {
        for e in &self.events {
            if e.event == event_name {
                return Some(e.clone());
            }
        }
        None
    }
}


// Check if in PerfStatData expr is detected with given value and unit.
#[cfg(test)]
macro_rules! assert_perf {
    ($data:expr, $event:expr, $expected_val:expr, $expected_unit:expr) => {
        let metric = $data.filter_value($event)
            .expect(&format!("Event '{}' not found", $event));
        assert!(
            (metric.value - $expected_val).abs() < 1e-6,
            "Event '{}' value mismatch: expected {}, got {}", 
            $event, $expected_val, metric.value
        );
        assert_eq!(
            metric.unit, $expected_unit, 
            "Event '{}' unit mismatch", $event
        );
    };
}

#[test]
fn test_perfstat_filter() {
    let data = r#"
{"counter-value":"0,374411","unit":"msec","event":"task-clock","event-runtime":374411,"pcnt-running":100.00,"metric-value":"0,000374","metric-unit":"CPUs utilized"}{"counter-value":"1,000000","unit":"","event":"context-switches","event-runtime":374411,"pcnt-running":100.00,"metric-value":"2,670862","metric-unit":"K/sec"}{"counter-value":"0,000000","unit":"","event":"cpu-migrations","event-runtime":374411,"pcnt-running":100.00,"metric-value":"0,000000","metric-unit":"/sec"}
{"counter-value":"75,000000","unit":"","event":"page-faults","event-runtime":374411,"pcnt-running":100.00,"metric-value":"200,314628","metric-unit":"K/sec"}
{"counter-value":"<not counted>","unit":"","event":"cpu_atom/cycles/","event-runtime":0,"pcnt-running":0.00,"metric-value":"0,000000","metric-unit":""}
{"counter-value":"1461835,000000","unit":"","event":"cpu_core/cycles/","event-runtime":374411,"pcnt-running":100.00,"metric-value":"3,904359","metric-unit":"GHz"}
"#;
    let perf_data: PerfStatData = data.parse().unwrap();

    assert_perf!(perf_data, "task-clock", 0.000374, "CPUs utilized");
    assert_perf!(perf_data, "context-switches", 2.670862, "K/sec");
    assert_perf!(perf_data, "cpu_atom/cycles/", 0.0, "");
    assert_perf!(perf_data, "cpu_core/cycles/", 3.904359, "GHz");
}

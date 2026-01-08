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

#[test]
fn test_perfstat_filter() {
    let data = r#"
{"counter-value":"0,374411","unit":"msec","event":"task-clock","event-runtime":374411,"pcnt-running":100.00,"metric-value":"0,000374","metric-unit":"CPUs utilized"}{"counter-value":"1,000000","unit":"","event":"context-switches","event-runtime":374411,"pcnt-running":100.00,"metric-value":"2,670862","metric-unit":"K/sec"}{"counter-value":"0,000000","unit":"","event":"cpu-migrations","event-runtime":374411,"pcnt-running":100.00,"metric-value":"0,000000","metric-unit":"/sec"}
{"counter-value":"75,000000","unit":"","event":"page-faults","event-runtime":374411,"pcnt-running":100.00,"metric-value":"200,314628","metric-unit":"K/sec"}
{"counter-value":"<not counted>","unit":"","event":"cpu_atom/cycles/","event-runtime":0,"pcnt-running":0.00,"metric-value":"0,000000","metric-unit":""}
{"counter-value":"1461835,000000","unit":"","event":"cpu_core/cycles/","event-runtime":374411,"pcnt-running":100.00,"metric-value":"3,904359","metric-unit":"GHz"}
"#;
    // TODO macro
    let perf_data: PerfStatData = data.parse().unwrap();

    // task-clock
    let task_clock = perf_data.filter_value("task-clock").unwrap();
    assert!((task_clock.value - 0.000374).abs() < 1e-6);
    assert_eq!(task_clock.unit, "CPUs utilized");

    // context-switches
    let context_switch = perf_data.filter_value("context-switches").unwrap();
    assert!((context_switch.value - 2.670862).abs() < 1e-6);
    assert_eq!(context_switch.unit, "K/sec");

    // cpu_atom/cycles/
    let cpu_atom = perf_data.filter_value("cpu_atom/cycles/").unwrap();
    assert!((cpu_atom.value - 0.0).abs() < 1e-6);
    assert_eq!(cpu_atom.unit, "");

    // cpu_core/cycles/
    let cpu_core = perf_data.filter_value("cpu_core/cycles/").unwrap();
    assert!((cpu_core.value - 3.904359).abs() < 1e-6);
    assert_eq!(cpu_core.unit, "GHz");
}

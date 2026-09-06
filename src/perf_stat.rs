use std::str::FromStr;

use serde::{Deserialize, Deserializer};

// Numbers returned by perf-stat can use dot or coma as decimal point, depending on system.
// This function parses to number, replacing comas with dots.
// Non-numeric values (e.g. "<not counted>") are reported as 0.
fn deserialize_perf_numbers<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(s.replace(',', ".").parse().unwrap_or(0.0))
}

#[derive(Debug, Deserialize, Clone)]
pub struct PerfEvent {
    #[serde(default)]
    pub event: String,

    // The normalized metric value/unit is plotted (e.g. "CPUs utilized"),
    // not the raw counter value.
    #[serde(rename = "metric-value", deserialize_with = "deserialize_perf_numbers", default)]
    pub value: f64,

    #[serde(rename = "metric-unit", default)]
    pub unit: String,
}

#[derive(Debug, Clone)]
pub struct PerfStatData {
    pub events: Vec<PerfEvent>,
}

impl FromStr for PerfStatData {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let events = serde_json::Deserializer::from_str(s)
            .into_iter::<PerfEvent>()
            .collect::<Result<Vec<_>, _>>()?;

        Ok(PerfStatData { events })
    }
}

impl PerfStatData {
    #[must_use]
    pub fn filter_value(&self, event_name: &str) -> Option<&PerfEvent> {
        self.events.iter().find(|e| e.event == event_name)
    }
}

#[cfg(test)]
mod tests {
    use crate::perf_stat::PerfStatData;
    // Check if in PerfStatData expr is detected with given value and unit.
    #[macro_export]
    macro_rules! assert_perf {
        ($data:expr, $event:expr, $expected_val:expr, $expected_unit:expr) => {
            let metric = $data
                .filter_value($event)
                .expect(&format!("Event '{}' not found", $event));
            assert!(
                (metric.value - $expected_val).abs() < 1e-6,
                "Event '{}' value mismatch: expected {}, got {}",
                $event,
                $expected_val,
                metric.value
            );
            assert_eq!(
                metric.unit, $expected_unit,
                "Event '{}' unit mismatch",
                $event
            );
        };
    }

    #[test]
    fn test_perfstat_filter_coma() {
        let data = r#"
    {"counter-value":"0,374411","unit":"msec","event":"task-clock","event-runtime":374411,"pcnt-running":100.00,"metric-value":"0,000374","metric-unit":"CPUs utilized"}{"counter-value":"1,000000","unit":"","event":"context-switches","event-runtime":374411,"pcnt-running":100.00,"metric-value":"2,670862","metric-unit":"K/sec"}{"counter-value":"0,000000","unit":"","event":"cpu-migrations","event-runtime":374411,"pcnt-running":100.00,"metric-value":"0,000000","metric-unit":"/sec"}
    {"counter-value":"75,000000","unit":"","event":"page-faults","event-runtime":374411,"pcnt-running":100.00,"metric-value":"200,314628","metric-unit":"K/sec"}
    {"counter-value":"<not counted>","unit":"","event":"cpu_atom/cycles/","event-runtime":0,"pcnt-running":0.00,"metric-value":"0,000000","metric-unit":""}
    {"counter-value":"1461835,000000","unit":"","event":"cpu_core/cycles/","event-runtime":374411,"pcnt-running":100.00,"metric-value":"3,904359","metric-unit":"GHz"}
    "#;
        let perf_data: PerfStatData = data.parse().unwrap();

        assert_perf!(perf_data, "task-clock", 0.000374, "CPUs utilized");
        assert_perf!(perf_data, "context-switches", 2.670862, "K/sec");
        assert_perf!(perf_data, "cpu-migrations", 0.0, "/sec");
        assert_perf!(perf_data, "page-faults", 200.314628, "K/sec");
        assert_perf!(perf_data, "cpu_atom/cycles/", 0.0, "");
        assert_perf!(perf_data, "cpu_core/cycles/", 3.904359, "GHz");
    }

    #[test]
    fn test_perfstat_filter_dot() {
        let data = r#"
    {"counter-value" : "0.598269", "unit" : "msec", "event" : "task-clock", "event-runtime" : 598269, "pcnt-running" : 100.00, "metric-value" : "0.422474", "metric-unit" : "CPUs utilized"}{"counter-value" : "2.000000", "unit" : "", "event" : "context-switches", "event-runtime" : 598269, "pcnt-running" : 100.00, "metric-value" : "3.342978", "metric-unit" : "K/sec"}
    {"counter-value" : "1.000000", "unit" : "", "event" : "cpu-migrations", "event-runtime" : 598269, "pcnt-running" : 100.00, "metric-value" : "1.671489", "metric-unit" : "K/sec"}{"counter-value" : "72.000000", "unit" : "", "event" : "page-faults", "event-runtime" : 598269, "pcnt-running" : 100.00, "metric-value" : "120.347202", "metric-unit" : "K/sec"}
    {"counter-value" : "1245859.000000", "unit" : "", "event" : "instructions", "event-runtime" : 598269, "pcnt-running" : 100.00, "metric-value" : "0.729721", "metric-unit" : "insn per cycle"}
    {"metric-value" : "0.515773", "metric-unit" : "stalled cycles per insn"}
    {"counter-value" : "1707308.000000", "unit" : "", "event" : 
    "cycles", "event-runtime" : 598269, "pcnt-running" : 100.00, "metric-value" : "2.853746", "metric-unit" : "GHz"}
    {"counter-value" : "642581.000000", "unit" : "", "event" : "stalled-cycles-frontend", "event-runtime" : 598269, "pcnt-running" : 100.00, "metric-value" : "37.637087", "metric-unit" : "frontend cycles idle", "metric-threshold" : "nearly bad"}
    {"counter-value" : "262792.000000", "unit" : "", "event" : "branches", "event-runtime" : 598269, "pcnt-running" : 100.00, "metric-value" : "439.253914", "metric-unit" : "M/sec"}{"counter-value" : "31359.000000", "unit" : "", "event" : "branch-misses", "event-runtime" : 598269, "pcnt-running" : 100.00, "metric-value" : "11.933012", "metric-unit" : "of all branches", "metric-threshold" : "nearly bad"}"#;
        let perf_data: PerfStatData = data.parse().unwrap();

        assert_perf!(perf_data, "task-clock", 0.422474, "CPUs utilized");
        assert_perf!(perf_data, "context-switches", 3.342978, "K/sec");
        assert_perf!(perf_data, "cpu-migrations", 1.671489, "K/sec");
        assert_perf!(perf_data, "page-faults", 120.347202, "K/sec");
        assert_perf!(perf_data, "instructions", 0.729721, "insn per cycle");
        assert_perf!(perf_data, "cycles", 2.853746, "GHz");
        assert_perf!(
            perf_data,
            "stalled-cycles-frontend",
            37.637087,
            "frontend cycles idle"
        );
        assert_perf!(perf_data, "branches", 439.253914, "M/sec");
        assert_perf!(perf_data, "branch-misses", 11.933012, "of all branches");
    }
}

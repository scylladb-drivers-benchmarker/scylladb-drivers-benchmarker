use std::{error::Error, str::FromStr};

use crate::utilities::BenchmarkPoint;

struct PerfStatData {

}


impl FromStr for PerfStatData {
    type Err = Box<dyn Error>;

    fn from_str(_: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl PerfStatData {
    fn filter(events: &str) -> BenchmarkPoint {
        todo!() // metric value
    }
}

trait GetMetricUnit {
    fn get_metric_unit(&self) -> String;
}

impl <T: Iterator<Item = PerfStatData>> GetMetricUnit for T {
    fn get_metric_unit(&self) -> String {
    // assert metric units are equal are equal
        todo!()
    }
}

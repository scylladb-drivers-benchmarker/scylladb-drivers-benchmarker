use super::data::PlottableValue;
use super::error::PlotError;

pub enum VisKind {
    Linear,
    Log,
}

pub(crate) trait SeriesValue: PlottableValue + PartialOrd + Into<f64> {}
impl<T: PlottableValue + PartialOrd + Into<f64>> SeriesValue for T {}

pub(crate) struct LinearSeries<T: SeriesValue> {
    pub y: Vec<Option<T>>,
}

pub(crate) struct LogSeries<T: SeriesValue> {
    pub y: Vec<Option<T>>,
}

pub(crate) enum ValueTransformation<T: SeriesValue> {
    Linear(LinearSeries<T>),
    Log(LogSeries<T>),
}

fn calc_range<T: SeriesValue>(iter: impl Iterator<Item = T>) -> Option<(T, T)> {
    let mut iter = iter;
    let first = iter.next()?;
    let (mut min, mut max) = (first.clone(), first);

    for v in iter {
        if v < min {
            min = v.clone()
        }
        if v > max {
            max = v.clone()
        }
    }

    Some((min, max))
}

impl<T: SeriesValue> LinearSeries<T> {
    fn series(&self) -> Vec<Option<f64>> {
        self.y
            .iter()
            .cloned()
            .map(|y| y.map(|v| v.into()))
            .collect()
    }

    fn range(&self) -> Option<(f64, f64)> {
        let (min, max) = calc_range(self.y.iter().filter_map(|v| v.clone()))?;
        Some((min.into(), max.into()))
    }
}

impl<T: SeriesValue> LogSeries<T> {
    fn series(&self) -> Result<Vec<Option<f64>>, PlotError> {
        let mut result = Vec::with_capacity(self.y.len());

        for y in &self.y {
            match y {
                Some(v) => {
                    let f = (*v).clone().into();
                    if f <= 0.0 {
                        return Err(PlotError::InvalidLogValue(f));
                    } else {
                        result.push(Some(f.log10()));
                    }
                }
                None => result.push(None),
            }
        }

        Ok(result)
    }

    fn range(&self) -> Result<Option<(f64, f64)>, PlotError> {
        if let Some((min, max)) = calc_range(self.y.iter().filter_map(|v| v.clone())) {
            let min_f = min.into();
            if min_f <= 0.0 {
                return Err(PlotError::InvalidLogValue(min_f));
            }

            let max_f = max.into();
            Ok(Some((min_f.log10(), max_f.log10())))
        } else {
            Ok(None)
        }
    }
}

impl<T: SeriesValue> ValueTransformation<T> {
    pub fn series(&self) -> Result<Vec<Option<f64>>, PlotError> {
        match self {
            ValueTransformation::Linear(s) => Ok(s.series()),
            ValueTransformation::Log(s) => s.series(),
        }
    }

    pub fn range(&self) -> Result<Option<(f64, f64)>, PlotError> {
        match self {
            ValueTransformation::Linear(s) => Ok(s.range()),
            ValueTransformation::Log(s) => s.range(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Clone, Debug, PartialOrd, Deserialize)]
    struct Dummy(f64);

    impl Into<f64> for Dummy {
        fn into(self) -> f64 {
            self.0
        }
    }

    impl PartialEq for Dummy {
        fn eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    #[test]
    fn linear_series() {
        let series = LinearSeries {
            y: vec![Some(Dummy(1.0)), Some(Dummy(3.0)), Some(Dummy(2.0))],
        };
        assert_eq!(series.series(), vec![Some(1.0), Some(3.0), Some(2.0)]);
        assert_eq!(series.range(), Some((1.0, 3.0)));

        let series: LinearSeries<Dummy> = LinearSeries {
            y: vec![None, None],
        };
        assert_eq!(series.series(), vec![None, None]);
        assert_eq!(series.range(), None);
    }

    #[test]
    fn log_series() {
        let series = LogSeries {
            y: vec![Some(Dummy(10.0)), Some(Dummy(1000.0)), Some(Dummy(100.0))],
        };
        assert_eq!(
            series.series().unwrap(),
            vec![Some(1.0), Some(3.0), Some(2.0)]
        );
        assert_eq!(series.range().unwrap(), Some((1.0, 3.0)));

        let series: LogSeries<Dummy> = LogSeries {
            y: vec![None, None],
        };
        assert_eq!(series.series().unwrap(), vec![None, None]);
        assert_eq!(series.range().unwrap(), None);

        let series = LogSeries {
            y: vec![Some(Dummy(-1.0))],
        };

        let range = series.range().unwrap_err();
        let series = series.series().unwrap_err();
        assert!(matches!(series, PlotError::InvalidLogValue(-1.0)));
        assert!(matches!(range, PlotError::InvalidLogValue(-1.0)));
    }

    #[test]
    fn value_transformation_linear() {
        let series = LinearSeries {
            y: vec![Some(Dummy(1.0)), Some(Dummy(3.0)), Some(Dummy(2.0))],
        };
        let vt = ValueTransformation::Linear(series);
        let vals = vt.series().unwrap();
        let range = vt.range().unwrap();

        assert_eq!(vals, vec![Some(1.0), Some(3.0), Some(2.0)]);
        assert_eq!(range, Some((1.0, 3.0)));

        let series: LinearSeries<Dummy> = LinearSeries {
            y: vec![None, None],
        };
        let vt = ValueTransformation::Linear(series);
        let vals = vt.series().unwrap();
        let range = vt.range().unwrap();

        assert_eq!(vals, vec![None, None]);
        assert_eq!(range, None);
    }

    #[test]
    fn value_transformation_log() {
        let series = LogSeries {
            y: vec![Some(Dummy(10.0)), Some(Dummy(1000.0)), Some(Dummy(100.0))],
        };
        let vt = ValueTransformation::Log(series);
        let vals = vt.series().unwrap();
        let range = vt.range().unwrap();

        assert_eq!(vals, vec![Some(1.0), Some(3.0), Some(2.0)]);
        assert_eq!(range, Some((1.0, 3.0)));

        let series: LogSeries<Dummy> = LogSeries {
            y: vec![None, None],
        };
        let vt = ValueTransformation::Log(series);
        let vals = vt.series().unwrap();
        let range = vt.range().unwrap();

        assert_eq!(vals, vec![None, None]);
        assert_eq!(range, None);

        let series = LogSeries {
            y: vec![Some(Dummy(0.0))],
        };
        let vt = ValueTransformation::Log(series);
        let vals = vt.series().unwrap_err();
        let range = vt.range().unwrap_err();

        assert!(matches!(vals, PlotError::InvalidLogValue(0.0)));
        assert!(matches!(range, PlotError::InvalidLogValue(0.0)));
    }
}

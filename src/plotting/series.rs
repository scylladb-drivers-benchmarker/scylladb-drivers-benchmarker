use super::data::PlottableValue;

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
    fn series(&self) -> Vec<Option<f64>> {
        self.y
            .iter()
            .cloned()
            .map(|y| y.map(|v| v.into().log10()))
            .collect()
    }

    fn range(&self) -> Option<(f64, f64)> {
        let (min, max) = calc_range(self.y.iter().filter_map(|v| v.clone()))?;
        Some((min.into().log10(), max.into().log10()))
    }
}

impl<T: SeriesValue> ValueTransformation<T> {
    pub fn series(&self) -> Vec<Option<f64>> {
        match self {
            ValueTransformation::Linear(s) => s.series(),
            ValueTransformation::Log(s) => s.series(),
        }
    }

    pub fn range(&self) -> Option<(f64, f64)> {
        match self {
            ValueTransformation::Linear(s) => s.range(),
            ValueTransformation::Log(s) => s.range(),
        }
    }
}

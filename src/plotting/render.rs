use super::error::PlotError;
use crate::utilities::{BenchmarkPoint, RangedCoordBenchmarkPoint};
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;

const LEGEND_LINE_LENGTH: i32 = 20;
const CROSS_SIZE: u32 = 5;

pub(crate) trait Renderable<'a, DB>
where
    DB: DrawingBackend + 'a,
    <DB as DrawingBackend>::ErrorType: 'static,
{
    fn add_to_plot(
        &self,
        chart: &mut ChartContext<'a, DB, Cartesian2d<RangedCoordBenchmarkPoint, RangedCoordf64>>,
    ) -> Result<(), PlotError>;
}

pub(crate) struct RenderableSeries {
    pub name: String,
    pub points: Vec<BenchmarkPoint>,
    pub series: Vec<Option<f64>>,
    pub color: PaletteColor<Palette99>,
    range: Option<(f64, f64)>,
}

impl RenderableSeries {
    pub(crate) fn new(
        name: String,
        points: Vec<BenchmarkPoint>,
        series: Vec<Option<f64>>,
        color: PaletteColor<Palette99>,
        range: Option<(f64, f64)>,
    ) -> Self {
        RenderableSeries {
            name,
            points,
            series,
            color,
            range,
        }
    }

    pub fn range(&self) -> Option<(f64, f64)> {
        self.range
    }
}

impl<'a, DB> Renderable<'a, DB> for RenderableSeries
where
    DB: DrawingBackend + 'a,
    <DB as DrawingBackend>::ErrorType: 'static,
{
    fn add_to_plot(
        &self,
        chart: &mut ChartContext<'a, DB, Cartesian2d<RangedCoordBenchmarkPoint, RangedCoordf64>>,
    ) -> Result<(), PlotError> {
        let color = self.color.to_rgba();
        let name = self.name.clone();
        let y_max = chart.as_coord_spec().y_spec().range().end;

        let mut line_points: Vec<(BenchmarkPoint, f64)> = Vec::new();

        for (&x, y_opt) in self.points.iter().zip(self.series.iter()) {
            match y_opt {
                Some(y) => line_points.push((x, *y)),
                None => {
                    line_points.push((x, y_max));
                    chart.draw_series(std::iter::once(Cross::new(
                        (x, y_max),
                        CROSS_SIZE,
                        color,
                    )))?;
                }
            }
        }

        chart
            .draw_series(LineSeries::new(line_points, color))?
            .label(name)
            .legend(move |(x, y)| {
                PathElement::new(vec![(x, y), (x + LEGEND_LINE_LENGTH, y)], color)
            });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn renderable_series_runs() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        let backend = BitMapBackend::new(&path, (640, 480)).into_drawing_area();
        backend.fill(&WHITE).unwrap();

        let mut chart = ChartBuilder::on(&backend)
            .margin(10)
            .build_cartesian_2d(0u64..10u64, 0f64..100f64)
            .unwrap();

        let series = RenderableSeries::new(
            "Test series".into(),
            (0..10).collect(),
            vec![
                Some(10.0),
                Some(20.0),
                Some(30.0),
                None,
                Some(50.0),
                Some(60.0),
                Some(70.0),
                Some(80.0),
                Some(90.0),
                Some(100.0),
            ],
            Palette99::pick(0),
            Some((10.0, 100.0)),
        );

        let result = series.add_to_plot(&mut chart);
        assert!(result.is_ok());
    }
}

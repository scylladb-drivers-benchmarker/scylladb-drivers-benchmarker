use crate::utilities::{BenchmarkPoint, RangedCoordBenchmarkPoint};
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;

pub(crate) trait Renderable<'a, BC> {
    fn add_to_plot(&self, backend_or_chart: &mut BC) -> Result<(), Box<dyn std::error::Error>>;
}

pub(crate) struct RenderableSeries {
    pub name: String,
    pub points: Vec<BenchmarkPoint>,
    pub series: Vec<Option<f64>>,
    pub color: PaletteColor<Palette99>,
    pub range: Option<(f64, f64)>,
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

impl<'a>
    Renderable<
        'a,
        ChartContext<'a, BitMapBackend<'a>, Cartesian2d<RangedCoordBenchmarkPoint, RangedCoordf64>>,
    > for RenderableSeries
{
    fn add_to_plot(
        &self,
        chart: &mut ChartContext<
            'a,
            BitMapBackend<'a>,
            Cartesian2d<RangedCoordBenchmarkPoint, RangedCoordf64>,
        >,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let color = self.color.to_rgba();
        let name = self.name.clone();
        let y_max = chart.as_coord_spec().y_spec().range().end;

        let mut line_points: Vec<(BenchmarkPoint, f64)> = Vec::new();

        for (&x, y_opt) in self.points.iter().zip(self.series.iter()) {
            match y_opt {
                Some(y) => line_points.push((x, *y)),
                None => {
                    chart.draw_series(std::iter::once(Cross::new((x, y_max / 2.0), 5, color)))?;
                }
            }
        }

        chart
            .draw_series(LineSeries::new(line_points, color))?
            .label(name)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color));

        chart
            .configure_series_labels()
            .border_style(BLACK)
            .background_style(WHITE.mix(0.8))
            .draw()?;

        Ok(())
    }
}

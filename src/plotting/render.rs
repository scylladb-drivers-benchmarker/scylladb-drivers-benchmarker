use super::error::PlotError;
use super::flamegraph_plot::ArtifactFile;
use crate::utilities::{BenchmarkPoint, RangedCoordBenchmarkPoint};
use html_escape::encode_safe;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const LEGEND_LINE_LENGTH: i32 = 20;
const CROSS_SIZE: u32 = 5;

pub(crate) trait Renderable<'a, DB>
where
    DB: DrawingBackend + 'a,
    <DB as DrawingBackend>::ErrorType: 'static,
{
    /// Draws the renderable onto the provided chart(s).
    ///
    /// # Panics
    ///
    /// Panics if `charts` is empty.
    fn add_to_plot(
        &self,
        charts: &mut [ChartContext<
            'a,
            DB,
            Cartesian2d<RangedCoordBenchmarkPoint, RangedCoordf64>,
        >],
    ) -> Result<(), PlotError>;
}

pub(crate) struct RenderableSeries {
    pub points: Vec<BenchmarkPoint>,
    name: String,
    series: Vec<Option<f64>>,
    color: PaletteColor<Palette99>,
    range: Option<(f64, f64)>,
}

pub(crate) struct RenderablePerfStat {
    pub points: Vec<BenchmarkPoint>,
    pub color: PaletteColor<Palette99>,
    pub name: String,
    values: Vec<Vec<Option<f64>>>,
    ranges: Vec<Option<(f64, f64)>>,
}

pub(crate) struct RenderableFlamegraph {
    pub points: Vec<BenchmarkPoint>,
    pub name: String,
    data: Vec<Option<String>>,
    output: PathBuf,
    artifact: ArtifactFile,
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
        charts: &mut [ChartContext<
            'a,
            DB,
            Cartesian2d<RangedCoordBenchmarkPoint, RangedCoordf64>,
        >],
    ) -> Result<(), PlotError> {
        assert!(
            !charts.is_empty(),
            "Expected at least one chart to render onto"
        );
        let chart = &mut charts[0];

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

impl RenderablePerfStat {
    pub(crate) fn new(
        name: String,
        points: Vec<BenchmarkPoint>,
        values: Vec<Vec<Option<f64>>>,
        color: PaletteColor<Palette99>,
        ranges: Vec<Option<(f64, f64)>>,
    ) -> Self {
        RenderablePerfStat {
            name,
            points,
            values,
            color,
            ranges,
        }
    }

    pub fn ranges(&self) -> &Vec<Option<(f64, f64)>> {
        &self.ranges
    }
}

impl<'a, DB> Renderable<'a, DB> for RenderablePerfStat
where
    DB: DrawingBackend + 'a,
    <DB as DrawingBackend>::ErrorType: 'static,
{
    fn add_to_plot(
        &self,
        charts: &mut [ChartContext<
            'a,
            DB,
            Cartesian2d<RangedCoordBenchmarkPoint, RangedCoordf64>,
        >],
    ) -> Result<(), PlotError> {
        assert!(
            !charts.is_empty(),
            "Expected at least one chart to render onto"
        );
        assert_eq!(
            charts.len(),
            self.values.len(),
            "Number of charts must match number of metrics"
        );

        let color = self.color.to_rgba();

        for (id, chart) in charts.iter_mut().enumerate() {
            let name = self.name.clone();
            let y_max = chart.as_coord_spec().y_spec().range().end;

            let mut line_points: Vec<(BenchmarkPoint, f64)> = Vec::new();

            for (&x, y_opt) in self.points.iter().zip(self.values[id].iter()) {
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
        }

        Ok(())
    }
}

impl RenderableFlamegraph {
    pub(crate) fn new(
        name: String,
        points: Vec<BenchmarkPoint>,
        data: Vec<Option<String>>,
        output: PathBuf,
        artifact: ArtifactFile,
    ) -> Self {
        RenderableFlamegraph {
            name,
            points,
            data,
            output,
            artifact,
        }
    }
}

impl<'a, DB> Renderable<'a, DB> for RenderableFlamegraph
where
    DB: DrawingBackend + 'a,
    <DB as DrawingBackend>::ErrorType: 'static,
{
    fn add_to_plot(
        &self,
        _charts: &mut [ChartContext<
            'a,
            DB,
            Cartesian2d<RangedCoordBenchmarkPoint, RangedCoordf64>,
        >],
    ) -> Result<(), PlotError> {
        let flame_svg = fs::read_to_string(self.artifact.path()).map_err(|e| {
            PlotError::from_io_with_path(e, self.artifact.path().display().to_string())
        })?;

        let escaped = encode_safe(flame_svg.as_str());

        let iframe = format!(
            r#"<iframe srcdoc='<!DOCTYPE html><html><body>{}</body></html>'
            style="width:100%; height:1080px; border:none"></iframe>"#,
            escaped
        );

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.output)
            .map_err(|e| PlotError::from_io_with_path(e, self.output.display().to_string()))?;

        writeln!(file, "{}", iframe)
            .map_err(|e| PlotError::from_io_with_path(e, self.output.display().to_string()))?;

        Ok(())
    }
}

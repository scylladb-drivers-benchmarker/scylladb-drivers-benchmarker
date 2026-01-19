use std::io::Write;
use std::path::PathBuf;

use fs_err as fs;
use html_escape::encode_safe;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use regex::Regex;

use crate::plotting::PlotError;
use crate::plotting::core::ArtifactFile;
use crate::utilities::{BenchmarkPoint, RangedCoordBenchmarkPoint};

const LEGEND_LINE_LENGTH: i32 = 20;
const CROSS_SIZE: u32 = 5;

pub trait Renderable<'a, DB>
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

pub struct RenderableSeries {
    pub points: Vec<BenchmarkPoint>,
    name: String,
    series: Vec<Option<f64>>,
    color: PaletteColor<Palette99>,
    range: Option<(f64, f64)>,
}

pub struct RenderablePerfStat {
    pub points: Vec<BenchmarkPoint>,
    pub color: PaletteColor<Palette99>,
    pub name: String,
    values: Vec<Vec<Option<f64>>>,
    ranges: Vec<Option<(f64, f64)>>,
}

pub struct RenderableFlameGraph {
    output: PathBuf,
    artifacts: Vec<ArtifactFile>,
}

impl RenderableSeries {
    pub fn new(
        name: String,
        points: Vec<BenchmarkPoint>,
        series: Vec<Option<f64>>,
        color: PaletteColor<Palette99>,
        range: Option<(f64, f64)>,
    ) -> Self {
        RenderableSeries {
            points,
            name,
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
            if let Some(y) = y_opt {
                line_points.push((x, *y));
            } else {
                line_points.push((x, y_max));
                chart.draw_series(std::iter::once(Cross::new((x, y_max), CROSS_SIZE, color)))?;
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
    pub fn new(
        name: String,
        points: Vec<BenchmarkPoint>,
        values: Vec<Vec<Option<f64>>>,
        color: PaletteColor<Palette99>,
        ranges: Vec<Option<(f64, f64)>>,
    ) -> Self {
        RenderablePerfStat {
            points,
            color,
            name,
            values,
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
                if let Some(y) = y_opt {
                    line_points.push((x, *y));
                } else {
                    line_points.push((x, y_max));
                    chart.draw_series(std::iter::once(Cross::new(
                        (x, y_max),
                        CROSS_SIZE,
                        color,
                    )))?;
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

impl RenderableFlameGraph {
    pub fn new(output: PathBuf, artifacts: Vec<ArtifactFile>) -> Self {
        RenderableFlameGraph { output, artifacts }
    }
}

impl<'a, DB> Renderable<'a, DB> for RenderableFlameGraph
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
        let size_re =
            Regex::new(r#"<svg\s+(version="[^"]+")\s+width="([^"]+)"\s+height="([^"]+)""#)
                .expect("Regex creation failed");

        for artifact in &self.artifacts {
            let mut flame_svg = fs::read_to_string(artifact.path())?;

            let mut captured_width = String::new();
            let mut captured_height = String::new();

            // Replace the svg pixel dimensions with relative iframe percentage size
            // and save the previous ones, to keep aspect ratio.
            flame_svg = size_re
                .replace(&flame_svg, |captures: &regex::Captures| {
                    captured_width = captures[2].to_owned();
                    captured_height = captures[3].to_owned();
                    format!("<svg {} width=\"100%\" height=\"100%\"", &captures[1])
                })
                .into_owned();

            if captured_width.is_empty() || captured_height.is_empty() {
                return Err(PlotError::Internal(
                    "Regex match failed in generated svg".into(),
                ));
            }

            let escaped = encode_safe(flame_svg.as_str());

            let iframe = format!(
                r#"<iframe srcdoc='&lt;!DOCTYPE html&gt;&lt;html&gt;&lt;body&gt;{escaped}&lt;&#47;body&gt;&lt;&#47;html&gt;'
                style="width:100%; aspect-ratio:{captured_width}/{captured_height}; border:none"></iframe>"#
            );

            let mut file = fs::OpenOptions::new().append(true).open(&self.output)?;

            writeln!(file, "{iframe}")?;
        }

        Ok(())
    }
}

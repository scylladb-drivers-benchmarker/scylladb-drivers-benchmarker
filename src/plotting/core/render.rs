use std::io::Write;
use std::path::PathBuf;

use fs_err as fs;
use html_escape::encode_safe;
use plotters::coord::types::RangedCoordf64;
use plotters::prelude::*;
use regex::Regex;

use crate::plotting::PlotError;
use crate::plotting::core::{ArtifactFile, LEGEND_AREA_SIZE};
use crate::utilities::{BenchmarkPoint, RangedCoordBenchmarkPoint};

const LINE_STROKE_WIDTH: u32 = 4;
const CROSS_SIZE: u32 = 10;
const POINT_RADIUS: u32 = 8;
const ERROR_BAR_CAP_HALF_PIXELS: i32 = 8;
const LEGEND_MARKER_HALF_HEIGHT: i32 = 12;

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
    error_bars: Vec<Option<(f64, f64)>>,
    color: PaletteColor<Palette99>,
    range: Option<(f64, f64)>,
    /// Set to `(series index, series count)` to draw this series as one column
    /// per point instead of a line. The x axis is then indexed in slots rather
    /// than benchmark points - see `RenderableSeries::slot`.
    column: Option<(usize, usize)>,
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
        error_bars: Vec<Option<(f64, f64)>>,
        color: PaletteColor<Palette99>,
        range: Option<(f64, f64)>,
    ) -> Self {
        RenderableSeries {
            points,
            name,
            series,
            error_bars,
            color,
            range,
            column: None,
        }
    }

    pub fn range(&self) -> Option<(f64, f64)> {
        self.range
    }

    /// Draws this series as columns, `series_index` of `series_count` within
    /// each group.
    pub fn draw_as_column(&mut self, series_index: usize, series_count: usize) {
        self.column = Some((series_index, series_count));
    }

    /// Slot occupied by this series' column in the `group_index`-th group.
    /// Each group spans `series_count + 1` slots: one per series, plus one that
    /// separates it from the next group.
    #[must_use]
    pub fn slot(group_index: usize, series_index: usize, series_count: usize) -> u64 {
        (group_index * (series_count + 1) + series_index) as u64
    }

    /// Width of one group in slots, including the separating one.
    #[must_use]
    pub fn group_width(series_count: usize) -> u64 {
        (series_count + 1) as u64
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

        if let Some((series_index, series_count)) = self.column {
            let mut columns = Vec::new();
            let mut crosses = Vec::new();

            for (group_index, y_opt) in self.series.iter().enumerate() {
                let slot = Self::slot(group_index, series_index, series_count);
                match y_opt {
                    Some(y) => columns.push(Rectangle::new(
                        [(slot, 0.0), (slot + 1, *y)],
                        color.filled(),
                    )),
                    // A missing point leaves a gap between columns, which alone
                    // would be hard to tell from a very short column.
                    None => crosses.push(Cross::new((slot, y_max), CROSS_SIZE, color)),
                }
            }

            if !crosses.is_empty() {
                chart.draw_series(crosses)?;
            }

            chart
                .draw_series(columns)?
                .label(name)
                .legend(move |(x, y)| {
                    Rectangle::new(
                        [(x, y - LEGEND_MARKER_HALF_HEIGHT), (x + LEGEND_AREA_SIZE as i32, y + LEGEND_MARKER_HALF_HEIGHT)],
                        color.filled(),
                    )
                });

            return Ok(());
        }

        // Compute cap half-width once from the axis mapping (same for all points).
        let cap_half: u64 = {
            let x_range = chart.as_coord_spec().x_spec().range();
            let px_start = chart.backend_coord(&(x_range.start, 0.0)).0 as f64;
            let px_end = chart.backend_coord(&(x_range.end, 0.0)).0 as f64;
            let data_range = (x_range.end - x_range.start) as f64;
            if data_range > 0.0 && (px_end - px_start).abs() > 0.0 {
                let ppu = (px_end - px_start) / data_range;
                (ERROR_BAR_CAP_HALF_PIXELS as f64 / ppu.abs()).ceil() as u64
            } else {
                0
            }
        };

        let mut line_points: Vec<(BenchmarkPoint, f64)> = Vec::new();
        let mut crosses = Vec::new();

        for (&x, y_opt) in self.points.iter().zip(self.series.iter()) {
            if let Some(y) = y_opt {
                line_points.push((x, *y));
            } else {
                line_points.push((x, y_max));
                crosses.push(Cross::new((x, y_max), CROSS_SIZE, color));
            }
        }

        // Batch all crosses into one draw_series call so only one SeriesAnno is created.
        if !crosses.is_empty() {
            chart.draw_series(crosses)?;
        }

        chart
            .draw_series(LineSeries::new(line_points, color.stroke_width(LINE_STROKE_WIDTH)))?
            .label(name)
            .legend(move |(x, y)| {
                PathElement::new(
                    vec![(x, y), (x + LEGEND_AREA_SIZE as i32, y)],
                    color.stroke_width(LINE_STROKE_WIDTH),
                )
            });

        // Batch all error bar segments into one draw_series call so only one SeriesAnno is created.
        let error_bar_segments: Vec<PathElement<(BenchmarkPoint, f64)>> = self
            .points
            .iter()
            .zip(self.error_bars.iter())
            .filter_map(|(&x, eb_opt)| eb_opt.map(|(y_lo, y_hi)| (x, y_lo, y_hi)))
            .flat_map(|(x, y_lo, y_hi)| {
                let style = color.stroke_width(LINE_STROKE_WIDTH);
                [
                    PathElement::new(vec![(x, y_lo), (x, y_hi)], style),
                    PathElement::new(
                        vec![(x.saturating_sub(cap_half), y_lo), (x + cap_half, y_lo)],
                        style,
                    ),
                    PathElement::new(
                        vec![(x.saturating_sub(cap_half), y_hi), (x + cap_half, y_hi)],
                        style,
                    ),
                ]
            })
            .collect();

        if !error_bar_segments.is_empty() {
            chart.draw_series(error_bar_segments)?;
        }

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
            let mut dot_points: Vec<(BenchmarkPoint, f64)> = Vec::new();

            for (&x, y_opt) in self.points.iter().zip(self.values[id].iter()) {
                if let Some(y) = y_opt {
                    line_points.push((x, *y));
                    dot_points.push((x, *y));
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
                .draw_series(LineSeries::new(line_points, color.stroke_width(LINE_STROKE_WIDTH)))?
                .label(name)
                .legend(move |(x, y)| {
                    PathElement::new(
                        vec![(x, y), (x + LEGEND_AREA_SIZE as i32, y)],
                        color.stroke_width(LINE_STROKE_WIDTH),
                    )
                });

            chart.draw_series(
                dot_points
                    .into_iter()
                    .map(|p| Circle::new(p, POINT_RADIUS, color.filled())),
            )?;
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

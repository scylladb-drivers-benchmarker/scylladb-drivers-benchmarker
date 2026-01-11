use super::error::PlotError;
use plotters::prelude::*;

pub(crate) const MARGIN_SIZE: u32 = 20;
pub(crate) const X_LABEL_AREA_SIZE: u32 = 40;
pub(crate) const Y_LABEL_AREA_SIZE: u32 = 70;

pub(crate) const FONT_FAMILY: &str = "sans-serif";
pub(crate) const TITLE_FONT_SIZE: u32 = 40;
pub(crate) const TITLE_FONT: (&str, u32) = (FONT_FAMILY, TITLE_FONT_SIZE);
pub(crate) const CAPTION_FONT_SIZE: u32 = 24;
pub(crate) const CAPTION_FONT: (&str, u32) = (FONT_FAMILY, CAPTION_FONT_SIZE);
pub(crate) const LABEL_FONT_SIZE: u32 = 16;
pub(crate) const LABEL_FONT: (&str, u32) = (FONT_FAMILY, LABEL_FONT_SIZE);

pub(crate) const BACKGROUND_COLOR: RGBColor = WHITE;
pub(crate) const LEGEND_BORDER_COLOR: RGBColor = BLACK;

pub(crate) const LEGEND_BORDER_SIZE: u32 = 1;

pub(crate) trait Plot {
    fn plot<DB: DrawingBackend + NamedBackend>(&self, backend: DB) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static;
}

// A very hacky way to differentiate backends, necessary as text has different pixel size.
pub(crate) trait NamedBackend {
    fn name(&self) -> &'static str;
}

impl<'a> NamedBackend for BitMapBackend<'a> {
    fn name(&self) -> &'static str {
        "bitmap"
    }
}

impl<'a> NamedBackend for SVGBackend<'a> {
    fn name(&self) -> &'static str {
        "svg"
    }
}

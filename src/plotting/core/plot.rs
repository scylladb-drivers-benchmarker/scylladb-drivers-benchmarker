use crate::plotting::error::PlotError;

use plotters::backend::DrawingBackend;
use plotters::prelude::*;
use plotters_backend::DrawingErrorKind;

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
    fn plot<DB: DrawingBackend + BackendWithKind>(&self, backend: DB) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static;

    fn name(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackendKind {
    Bitmap,
    Svg,
    Html,
}

pub(crate) struct NullBackend;

impl DrawingBackend for NullBackend {
    type ErrorType = std::convert::Infallible;

    fn get_size(&self) -> (u32, u32) {
        (0, 0)
    }

    fn ensure_prepared(&mut self) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }

    fn present(&mut self) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }

    fn draw_pixel(
        &mut self,
        _point: plotters_backend::BackendCoord,
        _color: plotters_backend::BackendColor,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }
}

// A very hacky way to differentiate backends, necessary as text has different pixel size.
pub(crate) trait BackendWithKind {
    fn kind(&self) -> BackendKind;
}

impl<'a> BackendWithKind for BitMapBackend<'a> {
    fn kind(&self) -> BackendKind {
        BackendKind::Bitmap
    }
}

impl<'a> BackendWithKind for SVGBackend<'a> {
    fn kind(&self) -> BackendKind {
        BackendKind::Svg
    }
}

impl BackendWithKind for NullBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Html
    }
}

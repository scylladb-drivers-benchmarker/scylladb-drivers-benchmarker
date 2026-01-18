use std::path::{Path, PathBuf};

use fs_err as fs;
use plotters::backend::DrawingBackend;
use plotters::prelude::*;
use plotters_backend::DrawingErrorKind;
use tempfile::NamedTempFile;

use crate::plotting::PlotError;

pub const MARGIN_SIZE: u32 = 20;
pub const X_LABEL_AREA_SIZE: u32 = 40;
pub const Y_LABEL_AREA_SIZE: u32 = 70;

pub const FONT_FAMILY: &str = "sans-serif";
pub const TITLE_FONT_SIZE: u32 = 40;
pub const TITLE_FONT: (&str, u32) = (FONT_FAMILY, TITLE_FONT_SIZE);
pub const CAPTION_FONT_SIZE: u32 = 24;
pub const CAPTION_FONT: (&str, u32) = (FONT_FAMILY, CAPTION_FONT_SIZE);
pub const LABEL_FONT_SIZE: u32 = 16;
pub const LABEL_FONT: (&str, u32) = (FONT_FAMILY, LABEL_FONT_SIZE);

pub const BACKGROUND_COLOR: RGBColor = WHITE;
pub const LEGEND_BORDER_COLOR: RGBColor = BLACK;

pub const LEGEND_BORDER_SIZE: u32 = 1;

pub trait Plot {
    fn plot<DB: DrawingBackend + BackendWithKind>(&self, backend: DB) -> Result<(), PlotError>
    where
        DB::ErrorType: 'static;

    fn name(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Bitmap,
    Svg,
    Html,
}

pub struct NullBackend;

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
pub trait BackendWithKind {
    fn kind(&self) -> BackendKind;
}

impl BackendWithKind for BitMapBackend<'_> {
    fn kind(&self) -> BackendKind {
        BackendKind::Bitmap
    }
}

impl BackendWithKind for SVGBackend<'_> {
    fn kind(&self) -> BackendKind {
        BackendKind::Svg
    }
}

impl BackendWithKind for NullBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Html
    }
}

// Struct to hold both tmp and normal files.
pub struct ArtifactFile {
    path: PathBuf,
    _tmp: Option<NamedTempFile>,
}

impl ArtifactFile {
    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        if !path.exists() {
            fs::File::create(&path)?;
        }

        Ok(Self { path, _tmp: None })
    }

    pub fn temp() -> Self {
        let tmp = NamedTempFile::new().expect("NamedTempFile creation failed");
        let path = tmp.path().to_path_buf();
        Self {
            path,
            _tmp: Some(tmp),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

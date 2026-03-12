pub mod data;
pub mod plot;
pub mod render;
pub mod series;

pub use data::BenchmarkDataset;
pub use plot::{
    ArtifactFile, BACKGROUND_COLOR, BackendKind, BackendWithKind, CAPTION_FONT, LABEL_FONT,
    LEGEND_AREA_SIZE, LEGEND_BORDER_COLOR, LEGEND_BORDER_SIZE, LEGEND_FONT, LEGEND_MARGIN,
    MARGIN_RIGHT, MARGIN_SIZE, MARGIN_TOP, NullBackend, Plot, TICK_FONT, TITLE_FONT,
    TITLE_MARGIN_TOP, X_LABEL_AREA_SIZE, Y_LABEL_AREA_SIZE,
};
pub use render::{Renderable, RenderableFlameGraph, RenderablePerfStat, RenderableSeries};
pub use series::{LinearSeries, LogSeries, SeriesValue, ValueTransformation, VisKind};

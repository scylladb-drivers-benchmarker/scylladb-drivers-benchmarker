pub mod data;
pub mod plot;
pub mod render;
pub mod series;

pub use data::BenchmarkDataset;
pub use plot::{
    BACKGROUND_COLOR, BackendKind, BackendWithKind, CAPTION_FONT, LABEL_FONT, LEGEND_BORDER_COLOR,
    LEGEND_BORDER_SIZE, MARGIN_SIZE, NullBackend, Plot, TITLE_FONT, X_LABEL_AREA_SIZE,
    Y_LABEL_AREA_SIZE,
};
pub use render::{Renderable, RenderableFlamegraph, RenderablePerfStat, RenderableSeries};
pub use series::{LinearSeries, LogSeries, SeriesValue, ValueTransformation, VisKind};

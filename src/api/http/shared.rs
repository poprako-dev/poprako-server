pub use metric::{MetricTotal, read_total, record_response};

mod metric;
/// Prometheus metrics HTTP integration.
pub mod prometheus;

pub use metric::{MetricTotal, read_total, record_response};

// Response metrics implementation used by middleware and health endpoints.
mod metric;
/// Prometheus metrics HTTP integration.
pub mod prometheus;

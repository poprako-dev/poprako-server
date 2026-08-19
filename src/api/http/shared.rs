/// Response metrics implementation used by middleware and health endpoints.
pub mod metric;
/// Prometheus metrics HTTP integration.
pub mod prometheus;

pub use crate::api::http::shared::metric::MetricTotal;

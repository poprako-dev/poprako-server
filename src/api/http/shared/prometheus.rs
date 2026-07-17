//! Prometheus recorder initialization and metric rendering.

use std::sync::OnceLock;

use anyhow::Context as _;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Installs the global Prometheus recorder and retains its rendering handle.
pub fn init_prometheus() -> anyhow::Result<()> {
    //
    let prometheus_handle = PrometheusBuilder::new()
        .with_recommended_naming(true)
        .install_recorder()
        .context("failed to install Prometheus metrics recorder")?;

    PROMETHEUS_HANDLE.set(prometheus_handle).map_err(|_| {
        anyhow::anyhow!("Prometheus metrics recorder initialized twice")
    })
}

/// Renders the current metric registry in Prometheus text format.
pub fn render_detailed_metrics() -> Option<String> {
    PROMETHEUS_HANDLE.get().map(PrometheusHandle::render)
}

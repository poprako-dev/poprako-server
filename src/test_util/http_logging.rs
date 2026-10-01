//! Captures real HTTP responses and correlated logs with the production filter.

use std::io::Write;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use tower::ServiceExt as _;
use tracing::instrument::WithSubscriber as _;

use crate::api::http::middleware::trace;
use crate::log::{RequestLogFormat, request_log_filter};

#[derive(Clone, Default)]
pub struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl LogBuffer {
    pub fn contents(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl Write for LogBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        //
        self.0.lock().unwrap().extend_from_slice(bytes);

        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub async fn capture_request(
    router: Router,
    request: Request<Body>,
) -> (StatusCode, String, String) {
    //
    let buffer = LogBuffer::default();

    let writer = buffer.clone();

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(request_log_filter(Some("info")))
        .with_ansi(false)
        .event_format(RequestLogFormat::new(()))
        .with_writer(move || writer.clone())
        .finish();

    let (status, body) = async {
        //
        let response = router
            .layer(trace::trace_request())
            .layer(trace::set_request_id())
            .oneshot(request)
            .await
            .unwrap();

        let status = response.status();

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();

        (status, String::from_utf8(body.to_vec()).unwrap())
    }
    .with_subscriber(subscriber)
    .await;

    let logs = buffer.contents();

    (status, body, logs)
}

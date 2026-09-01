//! Application router assembly.
//!
//! Health lives at `/api/health`. Public auth routes (`/auth/register`,
//! `/auth/login`, `/auth/logout`) and all protected business routes live
//! under `/api/v1`. Protected routes are wrapped by the authorization
//! middleware.

// Versioned API route builders.
mod v1;

use axum::Router;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::get;
use tower_http::compression::CompressionLayer;

use crate::api::http::handler::health;
use crate::api::http::middleware::auth::authorize;
use crate::api::http::middleware::cors::cors;
use crate::api::http::middleware::metric::record_response_metric;
use crate::api::http::middleware::rate_limit::rate_limit;
use crate::api::http::middleware::trace::{
    propagate_request_id, set_request_id, trace_request,
};

#[cfg(feature = "swagger")]
use crate::api::http::openapi::ApiDoc;

use crate::api::http::state::AppHarn;

/// Builds the application router from the production harness.
pub fn new(harn: AppHarn) -> Router<AppHarn> {
    //
    let v1_protected = v1::v1_user_router()
        .merge(v1::v1_team_router())
        .merge(v1::v1_member_router())
        .merge(v1::v1_member_invitation_router())
        .merge(v1::v1_workset_router())
        .merge(v1::v1_comic_router())
        .merge(v1::v1_comic_archive_router())
        .merge(v1::v1_chapter_router())
        .merge(v1::v1_page_router())
        .merge(v1::v1_unit_router())
        .merge(v1::v1_assignment_router())
        .merge(v1::v1_assignment_invitation_router())
        .merge(v1::v1_system_mail_router())
        .merge(v1::v1_announcement_router())
        .merge(v1::v1_comment_router())
        .merge(v1::v1_termbase_router())
        .merge(v1::v1_term_router())
        .layer(from_fn_with_state(harn, authorize));

    let router = Router::new()
        .nest("/api/v1", v1::v1_public_router().merge(v1_protected))
        .route("/api/health", get(health::check_health))
        .route(
            "/api/health/detailed-metrics",
            get(health::detailed_metrics),
        )
        // .layer(from_fn(log_latency))
        .layer(from_fn(rate_limit))
        .layer(propagate_request_id())
        .layer(trace_request())
        .layer(set_request_id())
        .layer(from_fn(record_response_metric))
        .layer(cors())
        .layer(CompressionLayer::new());

    // Swagger UI — debug builds only
    #[cfg(feature = "swagger")]
    let router = {
        //
        use utoipa::OpenApi as _;

        use utoipa_swagger_ui::SwaggerUi;

        router.merge(
            SwaggerUi::new("/api/swagger-ui")
                .url("/api/openapi.json", ApiDoc::openapi()),
        )
    };

    router
}

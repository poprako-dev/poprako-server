use axum::Router;
use axum::routing::{get, post, put};

use crate::api::http::middleware::authorize::AuthorizeLayer;
use crate::api::http::middleware::latency::LogLatencyLayer;
use crate::api::http::middleware::trace::IdTraceLayer;
use crate::api::http::handler::authorization;
use crate::api::http::handler::health;
use crate::api::http::handler::member;
use crate::api::http::handler::team;
use crate::api::http::handler::user;
use crate::api::http::handler::workset;
use crate::api::http::openapi::ApiDoc;
use crate::harness::Harness;

pub fn new(harn: Harness) -> Router<Harness> {
    // Public routes — no authorization required.
    let v1_public = Router::new()
        .route("/check-health", get(health::check_health))
        .route("/auth/sign-up", post(authorization::sign_up))
        .route("/auth/sign-in", post(authorization::sign_in));

    // Protected sub-routers — each resource group requires a valid authorization token.
    let v1_user = Router::new()
        .route("/users/me", get(user::get_my_info).put(user::update_info))
        .route("/users/{user_id}", get(user::get_info))
        .route(
            "/users/{user_id}/avatar/reserve",
            post(user::reserve_avatar),
        )
        .route(
            "/users/{user_id}/avatar/mark-uploaded",
            post(user::mark_avatar_uploaded),
        );

    let v1_team = Router::new()
        .route("/teams", get(team::list).post(team::create))
        .route(
            "/teams/{team_id}",
            get(team::get_info).put(team::update_info).delete(team::delete),
        )
        .route(
            "/teams/{team_id}/avatar/reserve",
            post(team::reserve_avatar),
        )
        .route(
            "/teams/{team_id}/avatar/mark-uploaded",
            post(team::mark_avatar_uploaded),
        );

    let v1_member = Router::new()
        .route("/members", get(member::list_infos).post(member::create))
        .route("/members/mine", get(member::list_mine))
        .route("/members/join", post(member::join))
        .route("/members/detail", get(member::list_my_members))
        .route(
            "/members/{member_id}",
            put(member::update_roles).delete(member::delete),
        );

    let v1_workset = Router::new()
        .route("/worksets", get(workset::list).post(workset::create))
        .route(
            "/worksets/{workset_id}",
            put(workset::update).delete(workset::delete),
        );

    // Merge all protected sub-routers and apply authorization layer.
    let v1_protected = v1_user
        .merge(v1_team)
        .merge(v1_member)
        .merge(v1_workset)
        .layer(AuthorizeLayer::new(harn));

    // Nest all v1 routes under /api/v1, wrapped with request-id + tracing + latency logging.
    let mut router = Router::new()
        .nest("/api/v1", v1_public.merge(v1_protected))
        .layer(IdTraceLayer::new())
        .layer(LogLatencyLayer::new());

    // Swagger UI — only available in debug builds.
    #[cfg(debug_assertions)]
    {
        use utoipa::OpenApi as _;
        use utoipa_swagger_ui::SwaggerUi;

        router = router
            .merge(SwaggerUi::new("/api/swagger-ui").url("/api/openapi.json", ApiDoc::openapi()));
    }

    router
}

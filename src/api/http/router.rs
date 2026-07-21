//! Application router assembly.
//!
//! Health lives at `/api/health`. Public auth routes (`/auth/register`,
//! `/auth/login`, `/auth/logout`) and all protected business routes live
//! under `/api/v1`. Protected routes are wrapped by the authorization
//! middleware.

use axum::Router;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::{delete, get, post, put};

use crate::api::http::handler::{
    announcement, assignment, assignment_invitation, auth, chapter,
    chapter_port, comic, comment, health, member, member_invitation, page,
    system_mail, team, term, termbase, unit, user, workset,
};
use crate::api::http::middleware::auth::authorize;
use crate::api::http::middleware::metric::record_response_metric;
use crate::api::http::middleware::rate_limit::rate_limit;
use crate::api::http::middleware::trace::{
    propagate_request_id, set_request_id, trace_request,
};
#[cfg(feature = "swagger-ui")]
use crate::api::http::openapi::ApiDoc;
use crate::api::http::state::AppHarn;

/// Builds the application router from the production harness.
pub fn new(harn: AppHarn) -> Router<AppHarn> {
    //
    let v1_auth = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout));

    let v1_public = v1_auth;

    let v1_user = Router::new()
        .route("/users/me", get(user::get_my_info))
        .route(
            "/users/{user_id}",
            get(user::get_info)
                .put(user::update_info)
                .delete(user::delete),
        )
        .route("/users/{user_id}/password", put(user::update_password))
        .route(
            "/users/{user_id}/avatar/reserve",
            post(user::reserve_avatar),
        )
        .route(
            "/users/{user_id}/avatar/mark-uploaded",
            post(user::mark_avatar_uploaded),
        );

    let v1_team = Router::new()
        .route("/teams", get(team::list_infos).post(team::create))
        .route(
            "/teams/{team_id}",
            get(team::get_info)
                .put(team::update_info)
                .delete(team::delete),
        )
        .route(
            "/teams/{team_id}/avatar/reserve",
            post(team::reserve_avatar),
        )
        .route(
            "/teams/{team_id}/avatar/mark-uploaded",
            post(team::mark_avatar_uploaded),
        )
        .route(
            "/teams/{team_id}/member-invitations",
            get(member_invitation::list_infos),
        )
        .route("/teams/{team_id}/worksets", get(workset::list_infos))
        .route("/teams/{team_id}/termbases", get(termbase::list_team_infos));

    let v1_member = Router::new()
        .route("/members", get(member::list_infos).post(member::create))
        .route("/members/me", get(member::list_my_infos))
        .route("/members/join", post(member::join))
        .route("/members/{member_id}/roles", put(member::update_roles))
        .route("/members/{member_id}", delete(member::delete));

    let v1_member_invitation = Router::new()
        .route("/member-invitations", post(member_invitation::create))
        .route(
            "/member-invitations/{member_invitation_id}/roles",
            put(member_invitation::update_roles),
        )
        .route(
            "/member-invitations/{member_invitation_id}",
            delete(member_invitation::delete),
        );

    let v1_workset = Router::new()
        .route("/worksets", post(workset::create))
        .route(
            "/worksets/{workset_id}",
            get(workset::get_info)
                .put(workset::update_info)
                .delete(workset::delete),
        );

    let v1_comic = Router::new()
        .route("/comics", post(comic::create))
        .route("/worksets/{workset_id}/comics", get(comic::list_infos))
        .route(
            "/comics/{comic_id}/termbases",
            get(termbase::list_comic_infos),
        )
        .route(
            "/comics/{comic_id}",
            get(comic::get_info)
                .put(comic::update_info)
                .delete(comic::delete),
        )
        .route(
            "/comics/{comic_id}/cover/reserve",
            post(comic::reserve_cover),
        )
        .route(
            "/comics/{comic_id}/cover/mark-uploaded",
            post(comic::mark_cover_uploaded),
        )
        .route("/comics/{comic_id}/archive", post(comic::archive));

    let v1_chapter = Router::new()
        .route("/chapters", post(chapter::create))
        .route("/comics/{comic_id}/chapters", get(chapter::list_infos))
        .route(
            "/comics/{comic_id}/chapters/pinned",
            get(chapter::get_pinned),
        )
        .route(
            "/chapters/{chapter_id}",
            get(chapter::get_info)
                .patch(chapter::update_info)
                .delete(chapter::delete),
        )
        .route(
            "/chapters/{chapter_id}/stage/advance",
            post(chapter::advance_stage),
        )
        .route(
            "/chapters/{chapter_id}/translations/import",
            post(chapter_port::import),
        )
        .route(
            "/chapters/{chapter_id}/translations/export",
            get(chapter_port::export),
        )
        .route(
            "/chapters/{chapter_id}/translations/export/download",
            get(chapter_port::export_download),
        );

    let v1_page = Router::new()
        .route(
            "/chapters/{chapter_id}/pages",
            get(page::list_all_infos).delete(page::delete),
        )
        .route(
            "/chapters/{chapter_id}/pages/reserve",
            post(page::reserve_chapter_pages),
        )
        .route("/pages/{page_id}/image/reserve", post(page::reserve_image))
        .route(
            "/pages/{page_id}/image/mark-uploaded",
            post(page::mark_image_uploaded),
        );

    let v1_unit = Router::new()
        .route("/pages/{page_id}/units", get(unit::list_all_infos))
        .route("/pages/{page_id}/units/save", post(unit::save_infos));

    let v1_assignment = Router::new()
        .route("/assignments", get(assignment::list_infos))
        .route("/assignments/join", post(assignment::join))
        .route(
            "/chapters/{chapter_id}/assignments/{user_id}/roles",
            put(assignment::update_roles),
        )
        .route("/assignments/{assignment_id}", delete(assignment::delete));

    let v1_assignment_invitation = Router::new()
        .route(
            "/assignment-invitations",
            post(assignment_invitation::create),
        )
        .route(
            "/assignment-invitations/join",
            post(assignment_invitation::join),
        )
        .route(
            "/chapters/{chapter_id}/assignment-invitations",
            get(assignment_invitation::list_infos),
        )
        .route(
            "/assignment-invitations/{assignment_invitation_id}",
            delete(assignment_invitation::delete),
        );

    let v1_system_mail = Router::new()
        .route("/system-mails", get(system_mail::list_infos))
        .route("/system-mails/mark-read", post(system_mail::mark_read));

    let v1_announcement = Router::new()
        .route("/announcements", post(announcement::create))
        .route(
            "/teams/{team_id}/announcements",
            get(announcement::list_infos),
        );

    let v1_comment = Router::new()
        .route("/comments", post(comment::create))
        .route("/teams/{team_id}/comments", get(comment::list_infos));

    let v1_termbase = Router::new()
        .route("/termbases", post(termbase::create))
        .route(
            "/termbases/{termbase_id}",
            get(termbase::get_info)
                .put(termbase::update_info)
                .delete(termbase::delete),
        )
        .route("/termbases/{termbase_id}/terms", get(term::list_infos));

    let v1_term = Router::new().route("/terms", post(term::create)).route(
        "/terms/{term_id}",
        get(term::get_info)
            .put(term::update_info)
            .delete(term::delete),
    );

    let v1_protected = v1_user
        .merge(v1_team)
        .merge(v1_member)
        .merge(v1_member_invitation)
        .merge(v1_workset)
        .merge(v1_comic)
        .merge(v1_chapter)
        .merge(v1_page)
        .merge(v1_unit)
        .merge(v1_assignment)
        .merge(v1_assignment_invitation)
        .merge(v1_system_mail)
        .merge(v1_announcement)
        .merge(v1_comment)
        .merge(v1_termbase)
        .merge(v1_term)
        .layer(from_fn_with_state(harn.clone(), authorize));

    let router = Router::new()
        .nest("/api/v1", v1_public.merge(v1_protected))
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
        .layer(from_fn(record_response_metric));

    // Swagger UI — debug builds only
    #[cfg(feature = "swagger-ui")]
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

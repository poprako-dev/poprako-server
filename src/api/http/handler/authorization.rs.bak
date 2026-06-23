// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use axum::Json;
// use axum::extract::State;
// use axum::http::StatusCode;
// use cookie::{Cookie, SameSite};
// use tracing::instrument;
// 
// use crate::api::http::auth_token::AUTHORIZATION_COOKIE_NAME;
// use crate::api::http::result::{Accept as _, HttpError, HttpResult};
// use crate::harness::Harness;
// use crate::usecase_legacy;
// use crate::usecase_legacy::data_object::user::{
//     SignInParams, SignInReply, SignUpParams, SignUpReply,
// };
// 
// #[utoipa::path(
//     post,
//     path = "/auth/sign-up",
//     tag = "auth",
//     request_body = SignUpParams,
//     responses(
//         (status = 200, description = "Registration successful, sets auth cookie", body = SignUpReply),
//         (status = 400, description = "Invalid request parameters", body = HttpError),
//         (status = 409, description = "User already exists", body = HttpError)
//     )
// )]
// #[instrument(err, skip(harn, params))]
// pub async fn sign_up(
//     State(harn): State<Harness>,
//     Json(params): Json<SignUpParams>,
// ) -> HttpResult<SignUpReply> {
//     let reply = usecase_legacy::user::sign_up(&harn, params).await?;
// 
//     let cookie = Cookie::build((AUTHORIZATION_COOKIE_NAME, format!("Bearer {}", reply.token)))
//         .path("/")
//         .http_only(true)
//         .same_site(SameSite::Lax)
//         .build();
// 
//     reply
//         .accept(StatusCode::OK)
//         .map(|response| response.with_cookie(&cookie))
// }
// 
// #[utoipa::path(
//     post,
//     path = "/auth/sign-in",
//     tag = "auth",
//     request_body = SignInParams,
//     responses(
//         (status = 200, description = "Login successful, sets auth cookie", body = SignInReply),
//         (status = 400, description = "Invalid request parameters", body = HttpError),
//         (status = 401, description = "Invalid credentials", body = HttpError)
//     )
// )]
// #[instrument(err, skip(harn, params))]
// pub async fn sign_in(
//     State(harn): State<Harness>,
//     Json(params): Json<SignInParams>,
// ) -> HttpResult<SignInReply> {
//     let reply = usecase_legacy::user::sign_in(&harn, params).await?;
// 
//     let cookie = Cookie::build((AUTHORIZATION_COOKIE_NAME, format!("Bearer {}", reply.token)))
//         .path("/")
//         .http_only(true)
//         .same_site(SameSite::Lax)
//         .build();
// 
//     reply
//         .accept(StatusCode::OK)
//         .map(|response| response.with_cookie(&cookie))
// }

use axum::Json;
use axum::extract::State;

use crate::api::harness::Harness;
use crate::api::http::handler::HttpResult;
use crate::api::http::handler::result::Accept as _;
use crate::usecase;
use crate::usecase::value_object::user::{SignUpUserParams, SignUpUserReply};

pub async fn sign_up_user(
    State(harn): State<Harness>,
    Json(params): Json<SignUpUserParams>,
) -> HttpResult<SignUpUserReply> {
    usecase::user::sign_up_user(&harn, params).await?.accept()
}

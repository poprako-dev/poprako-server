use axum::Json;
use axum::extract::State;

use crate::api::harness::Harness;
use crate::api::http::handler::HttpResl;
use crate::api::http::handler::result::accept;
use crate::usecase;
use crate::usecase::value_object::user::{SignUpUserParams, SignUpUserReply};

pub async fn sign_up_user(
    State(harn): State<Harness>,
    Json(params): Json<SignUpUserParams>,
) -> HttpResl<SignUpUserReply> {
    let reply = usecase::user::sign_up_user(&harn, params).await?;
    accept(reply)
}

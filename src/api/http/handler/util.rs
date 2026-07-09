//! Shared handler helpers: path/body id consistency, token target checks, and
//! reusable query extractors.

use serde::Deserialize;

use utoipa::IntoParams;

use poprako_util::i18n::trl;

use crate::api::http::result::HttpError;
use crate::model::user::UserToken;
use crate::result::ExpectedVariant;

/// Pagination query parameters for nested list endpoints where the parent id
/// is carried by the path.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct Pagination {
    pub offset: u64,
    pub limit: u64,
}

/// Ensures a path id matches the body id, returning `422` on mismatch.
pub fn ensure_path_matches_body_id(
    path_id: &str,
    body_id: &str,
) -> Result<(), HttpError> {
    if path_id != body_id {
        return Err(HttpError::unprocessable("path id does not match body id"));
    }

    Ok(())
}

/// Ensures a path user id matches the authenticated user, returning `403` on
/// mismatch. Used by token-only avatar handlers that take the target id from
/// the path.
pub fn ensure_current_user(
    path_user_id: &str,
    token: &UserToken,
) -> Result<(), HttpError> {
    if path_user_id != token.user_id {
        return Err(HttpError::expected(
            ExpectedVariant::Perm,
            &trl("error-forbidden"),
        ));
    }

    Ok(())
}

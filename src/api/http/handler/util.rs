//! Shared handler helpers: path/body id consistency, token target checks, and
//! reusable query extractors.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::IntoParams;

use poprako_util::i18n::trl;

use crate::api::http::result::HttpError;
use crate::model::shared::user::UserToken;
use crate::result::ExpectedVariant;

/// Pagination query parameters for nested list endpoints where the parent id
/// is carried by the path.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct Pagination {
    //
    /// Zero-based offset for paginated results.
    pub offset: u32,
    /// Maximum number of items to return.
    pub limit: u32,
}

/// Ensures a path id matches the body id, returning `422` on mismatch.
pub fn ensure_path_matches_body_id(
    path_id: &str,
    body_id: &str,
) -> Result<(), HttpError> {
    //
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
    //
    if path_user_id != token.user_id {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            path_user_id = %path_user_id,
            token_user_id = %token.user_id,
            "expected error: path user does not match authenticated user",
        );

        return Err(HttpError::expected(ExpectedVariant::Perm, &err_message));
    }

    Ok(())
}

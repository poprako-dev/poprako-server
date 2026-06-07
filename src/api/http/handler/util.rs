use poprako_util::i18n::trl;

use crate::api::http::result::HttpError;
use crate::domain::model::aggr::user::UserToken;
use crate::domain::result::ExpectedVariant;

pub fn ensure_current_user(user_id: &str, user_token: &UserToken) -> Result<(), HttpError> {
    if user_id == user_token.user_id {
        return Ok(());
    }

    Err(HttpError::expected(
        &ExpectedVariant::Authentication,
        &trl("error-unauthorized"),
    ))
}

use crate::domain::action;
use crate::domain::model::aggr::user::{User, UserForm};
use crate::domain::query::user;

pub trait RegisterUserCapablity: user::UserQeury {}

pub async fn register_user(
    capability: &mut impl RegisterUserCapablity,
    form: UserForm,
) -> action::Result<User> {
    capability.create(form).await?;

    unimplemented!()
}

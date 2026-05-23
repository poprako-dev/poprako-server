use crate::domain::actor::ActorRetVal;
use crate::domain::model::aggr::user::{User, UserForm};
use crate::domain::query::Transactional;
use crate::domain::query::user::UserQeury;

pub async fn register_user<H>(harn: &H, form: UserForm) -> ActorRetVal<User>
where
    H: Transactional + UserQeury,
{
    unimplemented!()
}

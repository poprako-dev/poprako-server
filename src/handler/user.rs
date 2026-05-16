use crate::domain::query::user::UserQeury;
use crate::handler::result::Result;
use crate::handler::val::user::User;

pub async fn get_user<Q>(mut query: Q, id: &str) -> Result<User>
where
    Q: UserQeury,
{
    let _ = query.get_by_id(id).await;
    unimplemented!()
}

use crate::domain::query::user::UserQeuryMut;
use crate::handler::result::Result;
use crate::handler::val::user::User;

pub async fn get_user<Q>(query: &mut Q, id: &str) -> Result<User>
where
    Q: UserQeuryMut,
{
    let _ = query.get_by_id(id).await;
    unimplemented!()
}

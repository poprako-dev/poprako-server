use diesel_async::AsyncPgConnection;

use crate::domain::model::aggr::user::{User, UserCredential, UserForm};
use crate::domain::query;

pub(crate) async fn get_by_id(conn: &mut AsyncPgConnection, id: &str) -> query::QueryResult<User> {
    todo!()
}

pub(crate) async fn get_credential_by_qid(
    conn: &mut AsyncPgConnection,
    id: &str,
) -> query::QueryResult<UserCredential> {
    todo!()
}

pub(crate) async fn create(
    conn: &mut AsyncPgConnection,
    form: &UserForm,
) -> query::QueryResult<User> {
    todo!()
}

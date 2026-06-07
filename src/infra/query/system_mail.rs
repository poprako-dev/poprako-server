use async_trait::async_trait;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::Level;
use tracing::instrument;

use crate::domain::model::aggr::system_mail::SystemMailForm;
use crate::domain::query::system_mail::SystemMailQuery;
use crate::domain::result::DomainResult;
use crate::infra::query::RdbQuery;
use crate::infra::query::entity::system_mail::SystemMailEntry;
use crate::infra::query::schema::t_system_mail::dsl::*;
use crate::submit_query;

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn send(conn: &mut AsyncPgConnection, form: &SystemMailForm) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();

    let entry = SystemMailEntry {
        f_id: &form.id,
        f_receiver_id: &form.receiver_id,
        f_title: &form.title,
        f_content: &form.content,
        f_created_at: now,
    };

    diesel::insert_into(t_system_mail)
        .values(&entry)
        .execute(conn)
        .await?;

    Ok(())
}

#[async_trait]
impl SystemMailQuery for RdbQuery {
    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn send(&self, form: &SystemMailForm) -> DomainResult<()> {
        submit_query!(self.pool, send, form)
    }
}

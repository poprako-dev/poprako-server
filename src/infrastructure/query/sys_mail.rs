use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use tracing::Level;
use tracing::instrument;

use crate::domain::model::aggregate::system_mail::SystemMailForm;
use crate::domain::query::system_mail::SystemMailQuery;
use crate::domain::result::DomainResult;
use crate::infrastructure::query::Query;
use crate::infrastructure::query::schema::t_system_mail;
use crate::submit_query;
use crate::util::err::ErrorTrace as _;

#[instrument(skip(conn), level = Level::DEBUG)]
pub async fn send(conn: &mut AsyncPgConnection, form: SystemMailForm) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();

    diesel::insert_into(t_system_mail::table)
        .values((
            t_system_mail::id.eq(&form.id),
            t_system_mail::receiver_id.eq(&form.receiver_id),
            t_system_mail::title.eq(&form.title),
            t_system_mail::content.eq(&form.content),
            t_system_mail::created_at.eq(now),
        ))
        .execute(conn)
        .await?;

    Ok(())
}

#[async_trait]
impl SystemMailQuery for Query {
    #[instrument(skip(self), level = Level::DEBUG)]
    async fn send(&self, form: SystemMailForm) -> DomainResult<()> {
        submit_query!(self.pool, send, form)
    }
}

use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use tracing::Level;
use tracing::instrument;

use crate::allocate_connection;
use crate::domain::model::aggregate::sys_mail::SysMailCre;
use crate::domain::result::DomainResult;
use crate::infrastructure::query::schema::t_system_mail;
use crate::infrastructure::query::Query;
use crate::util::err::ErrorTrace as _;

pub async fn send(conn: &mut AsyncPgConnection, cre: &SysMailCre) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();

    diesel::insert_into(t_system_mail::table)
        .values((
            t_system_mail::id.eq(&cre.id),
            t_system_mail::receiver_id.eq(&cre.receiver_id),
            t_system_mail::title.eq(&cre.title),
            t_system_mail::content.eq(&cre.content),
            t_system_mail::created_at.eq(now),
        ))
        .execute(conn)
        .await?;

    Ok(())
}

impl Query {
    /// Sends a system mail notification by inserting a row into `t_system_mail`.
    ///
    /// Acquires a connection from the pool and releases it after the insert.
    #[instrument(skip(self), level = Level::DEBUG)]
    pub async fn send_sys_mail(&self, cre: &SysMailCre) -> DomainResult<()> {
        let mut conn = allocate_connection!(self.pool, "Query::send_sys_mail");
        send(conn.as_mut(), cre).await
    }
}

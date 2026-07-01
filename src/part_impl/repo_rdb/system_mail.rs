//! RDB-backed system mail repository — [`Execute`] implementations.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::part::repo::step::system_mail::{
    ListInfosByIds, ListInfosByReceiverId, MarkRead, Send, SendBatch,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_rdb::entity::system_mail::{SystemMailEntry, SystemMailRow};
use crate::part_impl::repo_rdb::{RdbRepo, schema};
use crate::part_impl::shared_rdb::result::diesel;
use crate::result::RegularError;

#[async_trait]
impl<'a> Execute<Send<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &Send<'a>) -> Result<(), RegularError> {
        let mut conn = self.conn().await?;

        let entry = SystemMailEntry::from(step.form);

        diesel::insert_into(schema::t_system_mail::table)
            .values(&entry)
            .execute(conn.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Execute<SendBatch<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &SendBatch<'a>) -> Result<(), RegularError> {
        let mut conn = self.conn().await?;

        let entries: Vec<SystemMailEntry<'_>> =
            step.forms.iter().map(SystemMailEntry::from).collect();

        diesel::insert_into(schema::t_system_mail::table)
            .values(&entries)
            .execute(conn.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByReceiverId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfosByReceiverId<'a>,
    ) -> Result<<ListInfosByReceiverId<'_> as poprako_transactional::step::Step>::Output, Self::Error>
    {
        let mut conn = self.conn().await?;

        let mut query = schema::t_system_mail::table
            .filter(schema::t_system_mail::f_receiver_id.eq(step.receiver_id))
            .select(SystemMailRow::as_select())
            .into_boxed();

        match step.spec.read {
            Some(read) => {
                query = query.filter(schema::t_system_mail::f_read.eq(read));
            }
            None => {}
        }

        let rows: Vec<SystemMailRow> = query
            .order_by(schema::t_system_mail::f_created_at.desc())
            .offset(step.spec.offset as i64)
            .limit(step.spec.limit as i64)
            .load(conn.conn())
            .await
            .map_err(diesel)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByIds<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfosByIds<'a>,
    ) -> Result<<ListInfosByIds<'_> as poprako_transactional::step::Step>::Output, Self::Error>
    {
        let mut conn = self.conn().await?;

        let rows: Vec<SystemMailRow> = schema::t_system_mail::table
            .filter(schema::t_system_mail::f_id.eq_any(step.ids))
            .select(SystemMailRow::as_select())
            .load(conn.conn())
            .await
            .map_err(diesel)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait]
impl<'a> Execute<MarkRead<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &MarkRead<'a>) -> Result<(), RegularError> {
        let mut conn = self.conn().await?;

        diesel::update(
            schema::t_system_mail::table.filter(schema::t_system_mail::f_id.eq(step.id)),
        )
        .set(schema::t_system_mail::f_read.eq(true))
        .execute(conn.conn())
        .await
        .map_err(diesel)?;

        Ok(())
    }
}

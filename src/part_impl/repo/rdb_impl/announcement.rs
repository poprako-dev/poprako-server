//! RDB-backed announcement repository.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use poprako_transactional::advance::Advance;

use crate::model::announcement_model;
use crate::part::repo::announcement::{
    AnnouncementRepo, AnnouncementRepoTransactional,
};
use crate::part::repo::step::announcement::{Create, ListInfos};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::rdb_impl::entity::announcement::{
    AnnouncementEntry, AnnouncementRow,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, RdbRepoTransactional, incl};
use crate::part_impl::shared::result::diesel;
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{RegularError, RegularResult};

use crate::part_impl::repo::rdb_impl::schema::t_announcement::dsl::*;

impl AnnouncementRepo<RdbContext> for RdbRepo {}

impl AnnouncementRepoTransactional<RdbContext> for RdbRepoTransactional {}

/// Queries announcement rows filtered by team ID, ordered by creation time descending.
async fn list_infos(
    conn: &mut RdbConn,
    spec: &announcement_model::ListSpec,
) -> RegularResult<Vec<announcement_model::Info>> {
    //
    let rows: Vec<AnnouncementRow> = t_announcement
        .filter(f_team_id.eq(spec.team_id.as_str()))
        .select(AnnouncementRow::as_select())
        .order_by(f_created_at.desc())
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut infos: Vec<announcement_model::Info> =
        rows.into_iter().map(Into::into).collect();

    incl::announcement::populate_announcement_incls(
        conn,
        &mut infos,
        &spec.incl_opt,
    )
    .await?;

    Ok(infos)
}

/// Inserts a new announcement row from the given form and returns the created info.
async fn create(
    conn: &mut RdbConn,
    form: &announcement_model::Form,
) -> RegularResult<announcement_model::Info> {
    //
    let entry = AnnouncementEntry::from(form);

    let row: AnnouncementRow = diesel::insert_into(t_announcement)
        .values(&entry)
        .returning(AnnouncementRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> RegularResult<Vec<announcement_model::Info>> {
        submit_query!(self.core, list_infos, step.spec)
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> RegularResult<announcement_model::Info> {
        create(context.conn(), step.form).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;

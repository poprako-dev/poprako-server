//! RDB-backed announcement repository.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::announcement::{
    AnnouncementEntry, AnnouncementInfo, AnnouncementListSpec,
};
use crate::part::repo::announcement::AnnouncementRepo;
use crate::part::repo::oper::announcement::{
    CreateAnnouncement, ListAnnouncementInfos,
};
use crate::part_impl::repo::rdb_impl::entity::announcement::{
    AnnouncementRow, AnnouncementRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_announcement::dsl::*;
use crate::part_impl::repo::rdb_impl::{RdbRepo, incl};
use crate::part_impl::shared::result::diesel;
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{BaseError, BaseResult, accept};

#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

impl AnnouncementRepo<RdbContext> for RdbRepo {}

/// Queries announcement rows filtered by team ID, ordered by creation time descending.
#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &AnnouncementListSpec,
) -> BaseResult<Vec<AnnouncementInfo>> {
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

    let mut infos: Vec<AnnouncementInfo> =
        rows.into_iter().map(Into::into).collect();

    incl::announcement::populate_announcement_incls(
        conn,
        &mut infos,
        &spec.incl_opt,
    )
    .await?;

    accept(infos)
}

/// Inserts a new announcement row from the given entry and returns the created info.
#[instrument(level = "info", err(Debug), skip_all)]
async fn create(
    conn: &mut RdbConn,
    entry: &AnnouncementEntry,
) -> BaseResult<AnnouncementInfo> {
    //
    let entry = AnnouncementRowEntry::from(entry);

    let row: AnnouncementRow = diesel::insert_into(t_announcement)
        .values(&entry)
        .returning(AnnouncementRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    accept(row.into())
}

impl Run<ListAnnouncementInfos<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListAnnouncementInfos<'_>,
    ) -> BaseResult<Vec<AnnouncementInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Step<CreateAnnouncement<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateAnnouncement<'_>,
    ) -> BaseResult<AnnouncementInfo> {
        create(context.conn(), oper.entry).await
    }
}

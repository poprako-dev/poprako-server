//! RDB-backed announcement repository.

/// Announcement RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use crate::model::read::proj::announcement::AnnouncementInfo;
use crate::model::read::spec::announcement::AnnouncementListSpec;
use crate::model::write::announcement::AnnouncementEntry;
use crate::part::nucl::RepeatableRead;
use crate::part::repo::oper::announcement::{
    CreateAnnouncement, ListAnnouncementInfos,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::announcement::{
    AnnouncementEntryRow, AnnouncementInfoRow,
};
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::schema::t_announcement::dsl::*;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;
use crate::shared::{RdbConn, RdbContext};

// Queries announcement rows filtered by team ID, ordered by creation time descending.
#[instrument(level = "info", skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &AnnouncementListSpec,
) -> BaseRest<Vec<AnnouncementInfo>> {
    //
    let rows = t_announcement
        .filter(f_team_id.eq(spec.team_id.as_str()))
        .select(AnnouncementInfoRow::as_select())
        .order_by(f_created_at.desc())
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load::<AnnouncementInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let mut infos = rows
        .into_iter()
        .map(Into::into)
        .collect::<Vec<AnnouncementInfo>>();

    incl::announcement::populate_announcement_incls(
        conn,
        &mut infos,
        &spec.incl_opt,
    )
    .await?;

    accept(infos)
}

// Inserts a new announcement row from the given entry and returns the created info.
#[instrument(level = "info", skip_all)]
async fn create(
    conn: &mut RdbConn,
    entry: &AnnouncementEntry,
) -> BaseRest<AnnouncementInfo> {
    //
    let entry = AnnouncementEntryRow::from(entry);

    let row = diesel::insert_into(t_announcement)
        .values(&entry)
        .returning(AnnouncementInfoRow::as_returning())
        .get_result::<AnnouncementInfoRow>(conn)
        .await
        .map_err(diesel)?;

    accept(row.into())
}

impl Run<ListAnnouncementInfos<'_>> for HybRepo {
    // Error type for the Run trait impl on announcement list query.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Executes the announcement list query with the given operation spec.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListAnnouncementInfos<'_>,
    ) -> BaseRest<Vec<AnnouncementInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl<L> Step<CreateAnnouncement<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send,
    L: AtLeast<RepeatableRead>,
{
    // Error type for the Step trait impl on announcement creation.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Runs announcement creation within an existing transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateAnnouncement<'_>,
    ) -> BaseRest<AnnouncementInfo> {
        create(context.conn(), oper.entry).await
    }
}

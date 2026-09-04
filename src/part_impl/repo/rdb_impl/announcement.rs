//! RDB-backed announcement repository.

/// Announcement RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::Run;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::announcement::AnnouncementInfo;
use crate::model::read::spec::announcement::AnnouncementListSpec;
use crate::model::write::announcement::{AnnouncementEntry, AnnouncementRepl};
use crate::part::repo::oper::announcement::{
    CreateAnnouncement, DeleteAnnouncement, GetAnnouncementInfo,
    ListAnnouncementInfos, UpdateAnnouncement,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::announcement::{
    AnnouncementEntryRow, AnnouncementInfoRow,
};
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::schema::t_announcement::dsl::{
    f_content, f_created_at, f_id, f_team_id, f_title, t_announcement,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;

// Loads an announcement row by identifier.
#[instrument(level = "info", skip_all)]
async fn get_info(conn: &mut RdbConn, id: &str) -> BaseRest<AnnouncementInfo> {
    //
    let row = t_announcement
        .filter(f_id.eq(id))
        .select(AnnouncementInfoRow::as_select())
        .get_result::<AnnouncementInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        //
        let err_message = trl("error-announcement-not-found");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            announcement_id = %id,
            operation = "get announcement info",
            "expected error: announcement not found",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    };

    accept(row.into())
}

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
        .offset(i64::from(spec.offset))
        .limit(i64::from(spec.limit.get()))
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

// Replaces an announcement's editable fields.
#[instrument(level = "info", skip_all)]
async fn update_info(
    conn: &mut RdbConn,
    update: &AnnouncementRepl,
) -> BaseRest<()> {
    //
    diesel::update(t_announcement.filter(f_id.eq(&update.id)))
        .set((f_title.eq(&update.title), f_content.eq(&update.content)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Deletes an announcement row by identifier.
#[instrument(level = "info", skip_all)]
async fn delete_announcement(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    diesel::delete(t_announcement.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
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
        submit_query!(self.rdb_core, list_infos, oper.spec)
    }
}

impl Run<CreateAnnouncement<'_>> for HybRepo {
    // Error type for the announcement creation query.
    type Error = BaseError;

    // Creates an announcement independently.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &CreateAnnouncement<'_>,
    ) -> BaseRest<AnnouncementInfo> {
        submit_query!(self.rdb_core, create, oper.entry)
    }
}

impl Run<GetAnnouncementInfo<'_>> for HybRepo {
    // Error type for the announcement lookup query.
    type Error = BaseError;

    // Loads an announcement independently.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &GetAnnouncementInfo<'_>,
    ) -> BaseRest<AnnouncementInfo> {
        submit_query!(self.rdb_core, get_info, oper.id)
    }
}

impl Run<UpdateAnnouncement<'_>> for HybRepo {
    // Error type for the announcement update query.
    type Error = BaseError;

    // Updates an announcement independently.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &UpdateAnnouncement<'_>) -> BaseRest<()> {
        submit_query!(self.rdb_core, update_info, oper.update)
    }
}

impl Run<DeleteAnnouncement<'_>> for HybRepo {
    // Error type for the announcement deletion query.
    type Error = BaseError;

    // Deletes an announcement independently.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &DeleteAnnouncement<'_>) -> BaseRest<()> {
        submit_query!(self.rdb_core, delete_announcement, oper.id)
    }
}

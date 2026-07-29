//! RDB-backed member-invitation repository — free query functions and thin trait impls.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::member_invitation::{
    MemberInvitationEntry, MemberInvitationInfo, MemberInvitationListKind,
    MemberInvitationListSpec,
};
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::oper::member_invitation::{
    CreateMemberInvitation, DeleteMemberInvitation, GetMemberInvitationInfo,
    GetMemberInvitationInfoExcluded, ListMemberInvitationInfos,
    PurgeExpiredMemberInvitation, UpdateMemberInvitation,
};
use crate::part_impl::repo::rdb_impl::entity::member_invitation::{
    MemberInvitationAspect, MemberInvitationRow, MemberInvitationRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_member_invitation::dsl::*;
use crate::part_impl::repo::rdb_impl::{RdbRepo, incl};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{BaseError, BaseResult, accept};
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::RoleMask;

#[cfg(all(test, feature = "repo"))]
mod tests;

impl MemberInvitationRepo<RdbContext> for RdbRepo {}

// ── Free functions ──────────────────────────────────────────────────────────

/// Query member invitations matching the given list spec, with optional includes.
#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &MemberInvitationListSpec,
) -> BaseResult<Vec<MemberInvitationInfo>> {
    //
    let mut query = t_member_invitation
        .filter(f_team_id.eq(spec.team_id.as_str()))
        .select(MemberInvitationRow::as_select())
        .into_boxed();

    query = match &spec.kind {
        //
        MemberInvitationListKind::All => query,

        MemberInvitationListKind::Pending => query.filter(f_pending.eq(true)),

        MemberInvitationListKind::Used => query.filter(f_pending.eq(false)),
    };

    let rows: Vec<MemberInvitationRow> = query
        .order_by(f_created_at.desc())
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut infos = Vec::with_capacity(rows.len());

    for row in rows {
        infos.push(row.try_into()?);
    }

    incl::member_invitation::populate_member_invitation_incls(
        conn,
        &mut infos,
        &spec.incl_opt,
    )
    .await?;

    accept(infos)
}

/// Load a single invitation info by ID with optional includes.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[MemberInvitationInclOpt],
) -> BaseResult<MemberInvitationInfo> {
    //
    let row: MemberInvitationRow = t_member_invitation
        .filter(f_id.eq(id))
        .select(MemberInvitationRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-invitation-not-found"))?;

    let mut info: MemberInvitationInfo = row.try_into()?;

    incl::member_invitation::populate_member_invitation_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    accept(info)
}

/// Create a new member invitation and return its info.
#[instrument(level = "info", err(Debug), skip_all)]
async fn create(
    conn: &mut RdbConn,
    entry: &MemberInvitationEntry,
) -> BaseResult<MemberInvitationInfo> {
    //
    let entry = MemberInvitationRowEntry::from(entry);

    let row: MemberInvitationRow = diesel::insert_into(t_member_invitation)
        .values(&entry)
        .returning(MemberInvitationRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    row.try_into()
}

/// Look up a pending invitation by code.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_by_code(
    conn: &mut RdbConn,
    code: &str,
) -> BaseResult<MemberInvitationInfo> {
    //
    let row: MemberInvitationRow = t_member_invitation
        .filter(f_code.eq(code))
        .filter(f_pending.eq(true))
        .select(MemberInvitationRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-invitation-not-found"))?;

    row.try_into()
}

/// Look up a pending invitation by code, locking the row for update.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_by_code_excluded(
    conn: &mut RdbConn,
    code: &str,
) -> BaseResult<MemberInvitationInfo> {
    //
    let row: MemberInvitationRow = t_member_invitation
        .filter(f_code.eq(code))
        .filter(f_pending.eq(true))
        .select(MemberInvitationRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-invitation-not-found"))?;

    row.try_into()
}

/// Mark a pending invitation as used.
#[instrument(level = "info", err(Debug), skip_all)]
async fn mark_pending_as_used(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = MemberInvitationAspect::new(now).pending(false);

    diesel::update(t_member_invitation.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Update the roles on an existing invitation.
#[instrument(level = "info", err(Debug), skip_all)]
async fn update_info(
    conn: &mut RdbConn,
    id: &str,
    roles: RoleMask,
) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect =
        MemberInvitationAspect::new(now).role_mask(i64::from(u32::from(roles)));

    diesel::update(t_member_invitation.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Delete a member invitation by ID.
#[instrument(level = "info", err(Debug), skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    diesel::delete(t_member_invitation.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Deletes a member invitation only while it remains pending.
#[instrument(level = "info", err(Debug), skip_all)]
async fn purge_pending(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    diesel::delete(
        t_member_invitation
            .filter(f_id.eq(id))
            .filter(f_pending.eq(true)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

impl<'a> Run<ListMemberInvitationInfos<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListMemberInvitationInfos<'a>,
    ) -> BaseResult<Vec<MemberInvitationInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl<'a, 'b> Run<GetMemberInvitationInfo<'a, 'b>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetMemberInvitationInfo<'a, 'b>,
    ) -> BaseResult<MemberInvitationInfo> {
        match oper {
            //
            GetMemberInvitationInfo::Id { id, incls } => {
                submit_query!(self.core, get_info_by_id, id, incls)
            }

            GetMemberInvitationInfo::Code { code } => {
                submit_query!(self.core, get_info_by_code, code)
            }
        }
    }
}

impl<'a> Step<CreateMemberInvitation<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateMemberInvitation<'a>,
    ) -> BaseResult<MemberInvitationInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<'a, 'b> Step<GetMemberInvitationInfo<'a, 'b>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetMemberInvitationInfo<'a, 'b>,
    ) -> BaseResult<MemberInvitationInfo> {
        match oper {
            //
            GetMemberInvitationInfo::Id { id, incls } => {
                get_info_by_id(context.conn(), id, incls).await
            }

            GetMemberInvitationInfo::Code { code } => {
                get_info_by_code(context.conn(), code).await
            }
        }
    }
}

impl<'a> Step<UpdateMemberInvitation<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateMemberInvitation<'a>,
    ) -> BaseResult<()> {
        match oper {
            //
            UpdateMemberInvitation::Info { update } => {
                update_info(context.conn(), update.id.as_str(), update.roles)
                    .await
            }

            UpdateMemberInvitation::MarkUsed { id } => {
                mark_pending_as_used(context.conn(), id).await
            }
        }
    }
}

impl<'a> Step<GetMemberInvitationInfoExcluded<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetMemberInvitationInfoExcluded<'a>,
    ) -> BaseResult<MemberInvitationInfo> {
        match oper {
            GetMemberInvitationInfoExcluded::Code { code } => {
                get_info_by_code_excluded(context.conn(), code).await
            }
        }
    }
}

impl<'a> Step<DeleteMemberInvitation<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteMemberInvitation<'a>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}

impl<'a> Step<PurgeExpiredMemberInvitation<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &PurgeExpiredMemberInvitation<'a>,
    ) -> BaseResult<()> {
        purge_pending(context.conn(), oper.id).await
    }
}

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::assignment_invitation::AssignmentInvitationInfo;
use crate::part::repo::oper::assignment_invitation::{
    CreateAssignmentInvitation, DeleteAssignmentInvitations,
    GetAssignmentInvitationInfo, GetAssignmentInvitationInfoExcluded,
    ListAssignmentInvitationInfos, MarkAssignmentInvitationUsed,
    PurgeExpiredAssignmentInvitation,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::assignment_invitation::step_impl::{
    create, delete, delete_by_chapter_id, get_info_by_code_excluded,
    get_info_by_id, list_infos, mark_pending_as_used, purge_pending,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult};

impl Run<ListAssignmentInvitationInfos<'_>> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListAssignmentInvitationInfos<'_>,
    ) -> BaseResult<Vec<AssignmentInvitationInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}
impl Run<GetAssignmentInvitationInfo<'_>> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetAssignmentInvitationInfo<'_>,
    ) -> BaseResult<AssignmentInvitationInfo> {
        match oper {
            GetAssignmentInvitationInfo::Id { id } => {
                submit_query!(self.core, get_info_by_id, id)
            }
        }
    }
}
impl Step<CreateAssignmentInvitation<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateAssignmentInvitation<'_>,
    ) -> BaseResult<AssignmentInvitationInfo> {
        create(context.conn(), oper.entry).await
    }
}
impl Step<GetAssignmentInvitationInfo<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetAssignmentInvitationInfo<'_>,
    ) -> BaseResult<AssignmentInvitationInfo> {
        match oper {
            GetAssignmentInvitationInfo::Id { id } => {
                get_info_by_id(context.conn(), id).await
            }
        }
    }
}
impl Step<GetAssignmentInvitationInfoExcluded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetAssignmentInvitationInfoExcluded<'_>,
    ) -> BaseResult<AssignmentInvitationInfo> {
        get_info_by_code_excluded(context.conn(), oper.code).await
    }
}
impl Step<MarkAssignmentInvitationUsed<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &MarkAssignmentInvitationUsed<'_>,
    ) -> BaseResult<()> {
        mark_pending_as_used(context.conn(), oper.id).await
    }
}

impl Step<PurgeExpiredAssignmentInvitation<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &PurgeExpiredAssignmentInvitation<'_>,
    ) -> BaseResult<()> {
        purge_pending(context.conn(), oper.id).await
    }
}

impl Run<PurgeExpiredAssignmentInvitation<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &PurgeExpiredAssignmentInvitation<'_>,
    ) -> BaseResult<()> {
        submit_query!(self.core, purge_pending, oper.id)
    }
}

impl Step<DeleteAssignmentInvitations<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteAssignmentInvitations<'_>,
    ) -> BaseResult<()> {
        match oper {
            //
            DeleteAssignmentInvitations::Id { id } => {
                delete(context.conn(), id).await
            }

            DeleteAssignmentInvitations::Chapter { chapter_id } => {
                delete_by_chapter_id(context.conn(), chapter_id).await
            }
        }
    }
}

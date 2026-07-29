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
use crate::part_impl::repo::rdb_impl::assignment_invitation::{
    create, delete, delete_by_chapter_id, get_info_by_code_excluded,
    get_info_by_id, list_infos, mark_pending_as_used, purge_pending,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult};

impl<'a> Run<ListAssignmentInvitationInfos<'a>> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListAssignmentInvitationInfos<'a>,
    ) -> BaseResult<Vec<AssignmentInvitationInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}
impl<'a> Run<GetAssignmentInvitationInfo<'a>> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetAssignmentInvitationInfo<'a>,
    ) -> BaseResult<AssignmentInvitationInfo> {
        match oper {
            GetAssignmentInvitationInfo::Id { id } => {
                submit_query!(self.core, get_info_by_id, id)
            }
        }
    }
}
impl<'a> Step<CreateAssignmentInvitation<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateAssignmentInvitation<'a>,
    ) -> BaseResult<AssignmentInvitationInfo> {
        create(context.conn(), oper.entry).await
    }
}
impl<'a> Step<GetAssignmentInvitationInfo<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetAssignmentInvitationInfo<'a>,
    ) -> BaseResult<AssignmentInvitationInfo> {
        match oper {
            GetAssignmentInvitationInfo::Id { id } => {
                get_info_by_id(context.conn(), id).await
            }
        }
    }
}
impl<'a> Step<GetAssignmentInvitationInfoExcluded<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetAssignmentInvitationInfoExcluded<'a>,
    ) -> BaseResult<AssignmentInvitationInfo> {
        get_info_by_code_excluded(context.conn(), oper.code).await
    }
}
impl<'a> Step<MarkAssignmentInvitationUsed<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &MarkAssignmentInvitationUsed<'a>,
    ) -> BaseResult<()> {
        mark_pending_as_used(context.conn(), oper.id).await
    }
}

impl<'a> Step<PurgeExpiredAssignmentInvitation<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &PurgeExpiredAssignmentInvitation<'a>,
    ) -> BaseResult<()> {
        purge_pending(context.conn(), oper.id).await
    }
}

impl<'a> Step<DeleteAssignmentInvitations<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteAssignmentInvitations<'a>,
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

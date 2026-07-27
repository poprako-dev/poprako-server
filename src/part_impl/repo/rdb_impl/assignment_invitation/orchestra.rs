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
    // Non-transactional path that lists invitation infos for a list spec.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Delegate to `submit_query!` to run the list query outside transaction
    // boundaries and return all matching invitation summaries.
    async fn run(
        &self,
        oper: &ListAssignmentInvitationInfos<'_>,
    ) -> BaseResult<Vec<AssignmentInvitationInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}
impl Run<GetAssignmentInvitationInfo<'_>> for RdbRepo {
    // Non-transactional path for loading one invitation info by id.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Dispatch a single-id fetch through submit-query macro and return full info.
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
    // Create invitation rows and return resulting invitation info in tx scope.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Execute creation in the provided DB context and return created info payload.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateAssignmentInvitation<'_>,
    ) -> BaseResult<AssignmentInvitationInfo> {
        create(context.conn(), oper.entry).await
    }
}
impl Step<GetAssignmentInvitationInfo<'_>, RdbContext> for RdbRepo {
    // Transactional fetch for one invitation info by id.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Route a non-code lookup to `step_impl` and load exactly one record.
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
    // Transactional lookup for invitation by code while skipping soft-excluded rows.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Fetch by raw code and keep exclusion semantics required by this branch.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetAssignmentInvitationInfoExcluded<'_>,
    ) -> BaseResult<AssignmentInvitationInfo> {
        get_info_by_code_excluded(context.conn(), oper.code).await
    }
}
impl Step<MarkAssignmentInvitationUsed<'_>, RdbContext> for RdbRepo {
    // Transactional state transition that marks a pending invitation as used.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Perform state update for the given invitation id within the current tx.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &MarkAssignmentInvitationUsed<'_>,
    ) -> BaseResult<()> {
        mark_pending_as_used(context.conn(), oper.id).await
    }
}

impl Step<PurgeExpiredAssignmentInvitation<'_>, RdbContext> for RdbRepo {
    // Transactional delete/update behavior for purging expired invitations.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Purge expired invitation entries identified by invitation id.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &PurgeExpiredAssignmentInvitation<'_>,
    ) -> BaseResult<()> {
        purge_pending(context.conn(), oper.id).await
    }
}

impl Run<PurgeExpiredAssignmentInvitation<'_>> for RdbRepo {
    // Non-transactional interface for purging expired invitations.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Delegate expiration purge to a shared query execution helper.
    async fn run(
        &self,
        oper: &PurgeExpiredAssignmentInvitation<'_>,
    ) -> BaseResult<()> {
        submit_query!(self.core, purge_pending, oper.id)
    }
}

impl Step<DeleteAssignmentInvitations<'_>, RdbContext> for RdbRepo {
    // Transactional delete for invitation records.
    //
    // Deletes by invitation id or all invitations under a chapter.
    type Error = BaseError;
    #[instrument(level = "info", err(Debug), skip_all)]
    // Branch on request variant and execute the matching deletion statement.
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

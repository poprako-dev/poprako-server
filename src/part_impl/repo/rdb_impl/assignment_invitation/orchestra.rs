use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::assignment_invitation::AssignmentInvitationInfo;
use crate::part::repo::oper::assignment_invitation::{
    CreateAssignmentInvitation, DeleteAssignmentInvitations,
    GetAssignmentInvitationInfo, GetAssignmentInvitationInfoExcluded,
    ListAssignmentInvitationInfos, MarkAssignmentInvitationUsed,
    PurgeExpiredAssignmentInvitation,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::assignment_invitation::step_impl::{
    create, delete, delete_by_chapter_id, get_info_by_code_excluded,
    get_info_by_id, list_infos, mark_pending_as_used, purge_pending,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl Run<ListAssignmentInvitationInfos<'_>> for HybRepo {
    // Non-transactional path that lists invitation infos for a list spec.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Delegate to `submit_query!` to run the list query outside transaction
    // boundaries and return all matching invitation summaries.
    async fn run(
        &self,
        oper: &ListAssignmentInvitationInfos<'_>,
    ) -> BaseRest<Vec<AssignmentInvitationInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}
impl Run<GetAssignmentInvitationInfo<'_>> for HybRepo {
    // Non-transactional path for loading one invitation info by id.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Dispatch a single-id fetch through submit-query macro and return full info.
    async fn run(
        &self,
        oper: &GetAssignmentInvitationInfo<'_>,
    ) -> BaseRest<AssignmentInvitationInfo> {
        match oper {
            GetAssignmentInvitationInfo::Id { id } => {
                submit_query!(self.core, get_info_by_id, id)
            }
        }
    }
}
impl Step<CreateAssignmentInvitation<'_>, RdbContext> for HybRepo {
    // Create invitation rows and return resulting invitation info in tx scope.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Execute creation in the provided DB context and return created info payload.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateAssignmentInvitation<'_>,
    ) -> BaseRest<AssignmentInvitationInfo> {
        create(context.conn(), oper.entry).await
    }
}
impl Step<GetAssignmentInvitationInfo<'_>, RdbContext> for HybRepo {
    // Transactional fetch for one invitation info by id.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Route a non-code lookup to `step_impl` and load exactly one record.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetAssignmentInvitationInfo<'_>,
    ) -> BaseRest<AssignmentInvitationInfo> {
        match oper {
            GetAssignmentInvitationInfo::Id { id } => {
                get_info_by_id(context.conn(), id).await
            }
        }
    }
}
impl Step<GetAssignmentInvitationInfoExcluded<'_>, RdbContext> for HybRepo {
    // Transactional lookup for invitation by code while skipping soft-excluded rows.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Fetch by raw code and keep exclusion semantics required by this branch.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetAssignmentInvitationInfoExcluded<'_>,
    ) -> BaseRest<AssignmentInvitationInfo> {
        get_info_by_code_excluded(context.conn(), oper.code).await
    }
}
impl Step<MarkAssignmentInvitationUsed<'_>, RdbContext> for HybRepo {
    // Transactional state transition that marks a pending invitation as used.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Perform state update for the given invitation id within the current tx.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &MarkAssignmentInvitationUsed<'_>,
    ) -> BaseRest<()> {
        mark_pending_as_used(context.conn(), oper.id).await
    }
}

impl Step<PurgeExpiredAssignmentInvitation<'_>, RdbContext> for HybRepo {
    // Transactional delete/update behavior for purging expired invitations.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Purge expired invitation entries identified by invitation id.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &PurgeExpiredAssignmentInvitation<'_>,
    ) -> BaseRest<()> {
        purge_pending(context.conn(), oper.id).await
    }
}

impl Run<PurgeExpiredAssignmentInvitation<'_>> for HybRepo {
    // Non-transactional interface for purging expired invitations.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Delegate expiration purge to a shared query execution helper.
    async fn run(
        &self,
        oper: &PurgeExpiredAssignmentInvitation<'_>,
    ) -> BaseRest<()> {
        submit_query!(self.core, purge_pending, oper.id)
    }
}

impl Step<DeleteAssignmentInvitations<'_>, RdbContext> for HybRepo {
    // Transactional delete for invitation records.
    //
    // Deletes by invitation id or all invitations under a chapter.
    type Error = BaseError;
    #[instrument(level = "info", skip_all)]
    // Branch on request variant and execute the matching deletion statement.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteAssignmentInvitations<'_>,
    ) -> BaseRest<()> {
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

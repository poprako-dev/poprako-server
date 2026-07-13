use poprako_orchestra::{Run, Step};

use crate::model::assignment_invitation::AssignmentInvitationInfo;
use crate::part::repo::oper::assignment_invitation::{
    CreateAssignmentInvitation, DeleteAssignmentInvitations,
    GetAssignmentInvitationInfo, GetAssignmentInvitationInfoExcluded,
    ListAssignmentInvitationInfos, MarkAssignmentInvitationUsed,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::assignment_invitation::{
    create, delete, delete_by_chapter_id, get_info_by_code_excluded,
    get_info_by_id, list_infos, mark_pending_as_used,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{RegularError, RegularResult};

impl<'a> Run<ListAssignmentInvitationInfos<'a>> for RdbRepo {
    type Error = RegularError;
    async fn run(
        &self,
        oper: &ListAssignmentInvitationInfos<'a>,
    ) -> RegularResult<Vec<AssignmentInvitationInfo>> {
        submit_query!(
            self.core,
            list_infos,
            oper.chapter_id,
            oper.pending,
            oper.page.offset,
            oper.page.limit
        )
    }
}
impl<'a> Run<GetAssignmentInvitationInfo<'a>> for RdbRepo {
    type Error = RegularError;
    async fn run(
        &self,
        oper: &GetAssignmentInvitationInfo<'a>,
    ) -> RegularResult<AssignmentInvitationInfo> {
        match oper {
            GetAssignmentInvitationInfo::Id { id } => {
                submit_query!(self.core, get_info_by_id, id)
            }
        }
    }
}
impl<'a> Step<CreateAssignmentInvitation<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateAssignmentInvitation<'a>,
    ) -> RegularResult<AssignmentInvitationInfo> {
        create(context.conn(), oper.entry).await
    }
}
impl<'a> Step<GetAssignmentInvitationInfo<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetAssignmentInvitationInfo<'a>,
    ) -> RegularResult<AssignmentInvitationInfo> {
        match oper {
            GetAssignmentInvitationInfo::Id { id } => {
                get_info_by_id(context.conn(), id).await
            }
        }
    }
}
impl<'a> Step<GetAssignmentInvitationInfoExcluded<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetAssignmentInvitationInfoExcluded<'a>,
    ) -> RegularResult<AssignmentInvitationInfo> {
        get_info_by_code_excluded(context.conn(), oper.code).await
    }
}
impl<'a> Step<MarkAssignmentInvitationUsed<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &MarkAssignmentInvitationUsed<'a>,
    ) -> RegularResult<()> {
        mark_pending_as_used(context.conn(), oper.id).await
    }
}
impl<'a> Step<DeleteAssignmentInvitations<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteAssignmentInvitations<'a>,
    ) -> RegularResult<()> {
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

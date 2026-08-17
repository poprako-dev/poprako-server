//! Use case for listing immutable chapter workflow records.

use poprako_orchestra::{OperRun as _, run_proxy};
use tracing::instrument;

use crate::complex::chapter::ChapterPermComplex;
use crate::data::instr::chapter::ListChapterWorkflowRecordInfosInstr;
use crate::data::view::chapter_workflow_record::ChapterWorkflowRecordInfoView;
use crate::model::read::spec::chapter_workflow_record::ChapterWorkflowRecordListSpec;
use crate::model::shared::user::UserToken;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::chapter_workflow_record::ListChapterWorkflowRecordInfos;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseRest, accept};

/// Lists a chapter's immutable activity records in reverse chronological order.
#[instrument(level = "info", skip(repo))]
pub async fn list_workflow_record_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListChapterWorkflowRecordInfosInstr,
) -> BaseRest<Vec<ChapterWorkflowRecordInfoView>>
where
    C: poprako_orchestra::Context,
    R: ChapterWorkflowRecordRepo<C> + MemberRepo<C> + TeamRepo<C> + Sync,
{
    ChapterPermComplex::ensure_user_can_get_info(
        &mut run_proxy! {
            repo =>
                for<'a> ResolveTeamId<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &instr.chapter_id,
    )
    .await?;

    let spec = ChapterWorkflowRecordListSpec {
        chapter_id: instr.chapter_id,
        offset: instr.offset,
        limit: instr.limit,
    };

    let record_infos = ListChapterWorkflowRecordInfos { spec: &spec }
        .run_on(repo)
        .await?;

    accept(
        record_infos
            .into_iter()
            .map(ChapterWorkflowRecordInfoView::from)
            .collect(),
    )
}

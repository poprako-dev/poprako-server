//! Use case for listing immutable chapter workflow records.

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use crate::complex::chapter::ChapterPermComplex;
use crate::data::instr::chapter::ListChapterWorkflowRecordInfosInstr;
use crate::data::view::chapter_workflow_record::ChapterWorkflowRecordInfoView;
use crate::model::read::spec::chapter_workflow_record::ChapterWorkflowRecordListSpec;
use crate::model::shared::user::UserToken;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::chapter_workflow_record::ListChapterWorkflowRecordInfos;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseRest, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;

/// Lists a chapter's immutable activity records in reverse chronological order.
#[instrument(level = "info", skip(repo))]
pub async fn list_workflow_record_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListChapterWorkflowRecordInfosInstr,
) -> BaseRest<Vec<ChapterWorkflowRecordInfoView>>
where
    C: Context,
    R: ChapterWorkflowRecordRepo<C> + MemberRepo<C> + TeamRepo<C> + Sync,
{
    let member_info = MemberLoader::load_info_from_chapter(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &instr.chapter_id,
    )
    .await?;

    ChapterPermComplex::ensure_user_can_get_info(&member_info)?;

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

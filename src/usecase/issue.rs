//! Import and read the Chapter's single current review.

#[cfg(test)]
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use crate::complex::issue::perm as issue_perm_complex;
use crate::complex::{chapter as chapter_complex, issue as issue_complex};
use crate::data::instr::issue::ImportChapterIssuesInstr;
use crate::data::val::issue::ImportChapterIssuesVal;
use crate::data::view::issue::IssueInfoView;
use crate::model::shared::user::UserToken;
use crate::model::write::issue::{ChapterIssuesRepl, IssueEntry};
use crate::part::nucl::ReptRead;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::issue::IssueRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::issue::{ListIssueInfos, ReplaceChapterIssues};
use crate::part::repo::oper::page_artwork::ListPageArtworkInfos;
use crate::part::repo::page_artwork::PageArtworkRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::page::list as page_list_usecase;
use crate::util::next_snowflake_id;

/// Atomically replaces all issues without changing workflow or Unit counters.
#[instrument(level = "info", skip(nucl, repo, token, instr), fields(actor_user_id = %token.user_id, page_count = instr.pages.len()))]
pub async fn import_issue<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    chapter_id: String,
    instr: ImportChapterIssuesInstr,
) -> BaseRest<ImportChapterIssuesVal>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: ChapterRepo<C>
        + PageArtworkRepo<C>
        + AssignmentRepo<C>
        + IssueRepo<C>
        + ComicRepo<C>
        + Send
        + Sync,
{
    let imported = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let assignment_info = FindAssignmentInfo::ChapterUser {
                chapter_id: &chapter_info.id,
                user_id: &token.user_id,
            }
            .step_on(repo, context)
            .await?
            .ok_or_else(|| {
                //
                issue_complex::error(
                    ExpectedVariant::Perm,
                    "error-issue-reviewer-required",
                )
            })?;

            issue_perm_complex::ensure_user_can_import_issue(&assignment_info)?;

            chapter_complex::ensure_chapter_writable(&chapter_info)?;

            let page_artwork_infos = ListPageArtworkInfos {
                chapter_id: &chapter_info.id,
            }
            .step_on(repo, context)
            .await?;

            issue_complex::ensure_import_issue(&instr, &page_artwork_infos)?;

            let imported_page_count = instr.pages.len();

            let mut issue_entries = Vec::new();

            for page_instr in instr.pages {
                //
                for (index, issue_instr) in
                    page_instr.issues.into_iter().enumerate()
                {
                    let issue_entry = IssueEntry {
                        id: next_snowflake_id(),
                        page_artwork_id: page_instr.page_artwork_id.clone(),
                        index,
                        variant: issue_instr.variant,
                        layer_name: issue_instr.layer_name,
                        rect: issue_instr.rect.map(Into::into),
                        note: issue_instr.note,
                    };

                    issue_entries.push(issue_entry);
                }
            }

            let chapter_issues_repl = ChapterIssuesRepl {
                chapter_id: &chapter_info.id,
                entries: &issue_entries,
            };

            ReplaceChapterIssues {
                repl: &chapter_issues_repl,
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept(ImportChapterIssuesVal {
                imported_page_count,
                imported_issue_count: issue_entries.len(),
            })
        })
        .await?;

    accept(imported)
}

/// Reads a Chapter's issues ordered by composite page and issue position.
#[instrument(level = "info", skip(repo, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    chapter_id: String,
) -> BaseRest<Vec<IssueInfoView>>
where
    C: Context,
    R: IssueRepo<C> + TeamRepo<C> + MemberRepo<C> + AssignmentRepo<C> + Sync,
{
    page_list_usecase::ensure_user_can_list_infos::<C, R>(
        repo,
        &token,
        &chapter_id,
    )
    .await?;

    let issue_infos = ListIssueInfos {
        chapter_id: &chapter_id,
    }
    .run_on(repo)
    .await?;

    accept(issue_infos.into_iter().map(Into::into).collect())
}

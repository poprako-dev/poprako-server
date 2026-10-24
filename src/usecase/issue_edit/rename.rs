//! Atomic rename of the current chapter's category references.

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;

use crate::complex::issue_edit as issue_edit_complex;
use crate::data::instr::issue_edit::RenameIssueEditVariantInstr;
use crate::data::val::issue_edit::RenameIssueEditVariantVal;
use crate::model::shared::user::UserToken;
use crate::model::write::issue_edit::IssueEditVariantRename;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageArtworkImage;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::issue_edit::IssueEditRepo;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::issue_edit::{
    GetReviewVer, ListIssueVariantCounts, RenameIssueVariants,
};
use crate::result::{BaseError, BaseRest};
use crate::usecase::issue_edit::{access, document};

/// Renames all current references in one transaction without advancing workflow.
#[instrument(level = "info",
         skip(nucl,
         repo,
         obj_dept,
         token,
         instr),
         fields(actor_user_id = %token.user_id))]
pub async fn rename_variant<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    chapter_id: String,
    instr: RenameIssueEditVariantInstr,
) -> BaseRest<RenameIssueEditVariantVal>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: IssueEditRepo<C>
        + ChapterRepo<C>
        + AssignmentRepo<C>
        + ComicRepo<C>
        + Send
        + Sync,
    O: ObjDeptView<PageArtworkImage, C> + Send + Sync,
{
    let output = nucl
        .coord(async move |context| {
            //
            let chapter_info =
                access::write(repo, context, &chapter_id, &token.user_id)
                    .await?;

            let ver = GetReviewVer {
                chapter_id: &chapter_id,
            }
            .step_on(repo, context)
            .await?;

            if instr.review_key
                != issue_edit_complex::review_key(&chapter_id, ver)
                || instr.revision != ver.revision.to_string()
            {
                return Err(issue_edit_complex::conflict());
            }

            let counts = ListIssueVariantCounts {
                chapter_id: &chapter_id,
            }
            .step_on(repo, context)
            .await?;

            let rename = IssueEditVariantRename {
                chapter_id,
                from: instr.from.trim().to_owned(),
                to: instr.to.trim().to_owned(),
            };

            issue_edit_complex::ensure_rename(
                &rename.from,
                &rename.to,
                &counts,
            )?;

            let mut pages = Vec::new();

            if rename.from != rename.to {
                //
                let ids = RenameIssueVariants { rename: &rename }
                    .step_on(repo, context)
                    .await?;

                for id in ids {
                    //
                    pages.push(
                        document::page(
                            repo,
                            obj_dept,
                            context,
                            &rename.chapter_id,
                            &id,
                        )
                        .await?,
                    );
                }
            }

            if !pages.is_empty() {
                //
                TouchComicLastActive {
                    id: &chapter_info.comic_id,
                }
                .step_on(repo, context)
                .await?;
            }

            let summary =
                document::variants(repo, context, &rename.chapter_id).await?;

            accept!(RenameIssueEditVariantVal { summary, pages })
        })
        .await?;

    accept!(output)
}

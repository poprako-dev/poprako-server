//! Version-checked page replacement with bounded retry receipts.

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use sha2::{Digest as _, Sha256};
use tracing::instrument;
use uuid::Uuid;

use poprako_obj_dept::ObjDeptView;

use crate::complex::{
    issue as issue_complex, issue_edit as issue_edit_complex,
};
use crate::data::instr::issue_edit::SavePageIssuesInstr;
use crate::data::val::issue_edit::IssueEditPageVal;
use crate::model::shared::issue::IssueRect;
use crate::model::shared::user::UserToken;
use crate::model::write::issue::IssueEntry;
use crate::model::write::issue_edit::IssueEditPageRepl;
use crate::part::cache::issue_save::{IssueSaveCache, IssueSaveGuard as _};
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageArtworkImage;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::issue_edit::IssueEditRepo;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::issue_edit::{
    ListIssueInfosByIds, ReplacePageIssues,
};
use crate::part::repo::oper::page_artwork::GetPageArtworkInfo;
use crate::part::repo::page_artwork::PageArtworkRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::usecase::issue_edit::{access, document};

/// Replaces one page only when the caller still holds its editing baseline.
#[instrument(level = "info",
         skip(nucl,
         repo,
         cache,
         obj_dept,
         token,
         instr),
         fields(actor_user_id = %token.user_id))]
pub async fn save_infos<N, C, R, K, O>(
    (nucl, repo, cache, obj_dept): (&N, &R, &K, &O),
    token: UserToken,
    page_id: String,
    save_id: Uuid,
    instr: SavePageIssuesInstr,
) -> BaseRest<IssueEditPageVal>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: IssueEditRepo<C>
        + ChapterRepo<C>
        + PageArtworkRepo<C>
        + AssignmentRepo<C>
        + ComicRepo<C>
        + Send
        + Sync,
    K: IssueSaveCache + Sync,
    for<'save> K::Guard<'save>: Send + Sync,
    O: ObjDeptView<PageArtworkImage, C> + Send + Sync,
{
    let fingerprint = request_fingerprint(&token.user_id, &page_id, &instr)?;

    let guard = cache.lock_issue_save(save_id).await;

    let receipt = &guard;

    let page_info = GetPageArtworkInfo { id: &page_id }.run_on(repo).await?;

    let output = nucl
        .coord(async move |context| {
            //
            let chapter_info = access::write(
                repo,
                context,
                &page_info.chapter_id,
                &token.user_id,
            )
            .await?;

            let current = document::page(
                repo,
                obj_dept,
                context,
                &chapter_info.id,
                &page_id,
            )
            .await?;

            if current.review_key != instr.review_key
                || current.image_ver != instr.image_ver
            {
                return Err(issue_edit_complex::conflict());
            }

            if let Some(saved) = receipt.fingerprint() {
                //
                if saved != fingerprint {
                    //
                    return Err(issue_complex::error(
                        ExpectedVariant::Args,
                        "error-review-save-reused",
                    ));
                }

                return accept!(current);
            }

            if current.revision != instr.revision {
                return Err(issue_edit_complex::conflict());
            }

            let mut replacement =
                into_replacement(chapter_info.id, page_id, instr);

            preserve_and_validate(repo, context, &mut replacement).await?;

            ReplacePageIssues { repl: &replacement }
                .step_on(repo, context)
                .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            document::page(
                repo,
                obj_dept,
                context,
                &replacement.chapter_id,
                &replacement.page_artwork_id,
            )
            .await
        })
        .await?;

    if guard.fingerprint().is_none() {
        guard.record(fingerprint);
    }

    accept!(output)
}

// Hashes the actor, page and immutable payload.
fn request_fingerprint(
    user_id: &str,
    page_id: &str,
    instr: &SavePageIssuesInstr,
) -> BaseRest<[u8; 32]> {
    //
    let bytes =
        serde_json::to_vec(&(user_id, page_id, instr)).map_err(|source| {
            //
            unrecoverable_error! {
                message: "cannot fingerprint annotation batch".into();
                sdk_error = ?source,
                "annotation fingerprint serialization failed"
            }
        })?;

    accept!(Sha256::digest(bytes).into())
}

// Builds ordered entries from the submitted document.
fn into_replacement(
    chapter_id: String,
    page_artwork_id: String,
    instr: SavePageIssuesInstr,
) -> IssueEditPageRepl {
    //
    let entries = instr
        .issues
        .into_iter()
        .enumerate()
        .map(|(index, issue)| IssueEntry {
            id: issue.id,
            page_artwork_id: page_artwork_id.clone(),
            index,
            variant: issue.variant,
            layer_name: None,
            rect: issue.rect.map(|rect| IssueRect {
                x_coord: rect.x_coord,
                y_coord: rect.y_coord,
                width: rect.width,
                height: rect.height,
            }),
            note: issue.note,
        })
        .collect();

    IssueEditPageRepl {
        chapter_id,
        page_artwork_id,
        entries,
    }
}

// Validates identities and preserves existing legacy metadata.
async fn preserve_and_validate<C, R>(
    repo: &R,
    context: &mut C,
    replacement: &mut IssueEditPageRepl,
) -> BaseRest<()>
where
    C: Context + Send,
    R: IssueEditRepo<C> + Sync,
{
    let ids = replacement
        .entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<Vec<_>>();

    let existing = ListIssueInfosByIds { ids: &ids }
        .step_on(repo, context)
        .await?;

    issue_edit_complex::ensure_entries(
        &replacement.page_artwork_id,
        &replacement.entries,
        &existing,
    )?;

    for entry in &mut replacement.entries {
        //
        entry.layer_name = existing
            .iter()
            .find(|info| info.id == entry.id)
            .and_then(|info| info.layer_name.clone());
    }

    accept!(())
}

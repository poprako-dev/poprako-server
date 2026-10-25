//! Version-checked page replacement with bounded retry receipts.

use std::collections::HashSet;
use std::sync::Arc;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use sha2::{Digest as _, Sha256};
use tracing::instrument;
use uuid::Uuid;

use poprako_obj_dept::ObjDeptView;

use crate::complex::{
    issue as issue_complex, issue_edit as issue_edit_complex,
};
use crate::data::instr::issue_edit::SavePageIssuesInstr;
use crate::data::val::issue_edit::{CreatedIssueIdVal, SavePageIssuesVal};
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
use crate::util::next_business_id;
use crate::value::business_id::BusinessIdKind;
use crate::value::issue_save::{IssueSaveCreatedIssueId, IssueSaveReceipt};

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
) -> BaseRest<SavePageIssuesVal>
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

    let (page, created_issue_ids) = nucl
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

            if let Some(saved) = receipt.receipt() {
                //
                if saved.fingerprint != fingerprint {
                    //
                    return Err(issue_complex::error(
                        ExpectedVariant::Args,
                        "error-review-save-reused",
                    ));
                }

                return accept!((current, saved.created_issue_ids));
            }

            if current.revision != instr.revision {
                return Err(issue_edit_complex::conflict());
            }

            let (mut replacement, created_issue_ids) =
                into_replacement(chapter_info.id, page_id, instr)?;

            preserve_and_validate(
                repo,
                context,
                &mut replacement,
                &created_issue_ids,
            )
            .await?;

            ReplacePageIssues { repl: &replacement }
                .step_on(repo, context)
                .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            let page = document::page(
                repo,
                obj_dept,
                context,
                &replacement.chapter_id,
                &replacement.page_artwork_id,
            )
            .await?;

            accept!((
                page,
                Arc::<[IssueSaveCreatedIssueId]>::from(created_issue_ids)
            ))
        })
        .await?;

    if guard.receipt().is_none() {
        //
        guard.record(IssueSaveReceipt {
            fingerprint,
            created_issue_ids: created_issue_ids.clone(),
        });
    }

    accept!(SavePageIssuesVal {
        page,
        created_issue_ids: created_issue_ids
            .iter()
            .map(|pair| CreatedIssueIdVal {
                local_id: pair.local_id.clone(),
                issue_id: pair.issue_id.clone(),
            })
            .collect(),
    })
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
) -> BaseRest<(IssueEditPageRepl, Vec<IssueSaveCreatedIssueId>)> {
    //
    let mut local_ids = HashSet::new();

    let mut created_issue_ids = Vec::new();

    let mut entries = Vec::with_capacity(instr.issues.len());

    for (index, issue) in instr.issues.into_iter().enumerate() {
        //
        let id = match (issue.id, issue.local_id) {
            //
            (Some(id), None) if BusinessIdKind::Issue.is_valid_id(&id) => id,

            (None, Some(local_id))
                if !local_id.is_empty()
                    && local_ids.insert(local_id.clone()) =>
            {
                let issue_id = next_business_id(BusinessIdKind::Issue);

                created_issue_ids.push(IssueSaveCreatedIssueId {
                    local_id,
                    issue_id: issue_id.clone(),
                });

                issue_id
            }

            _ => {
                //
                return Err(issue_complex::error(
                    ExpectedVariant::Args,
                    "error-issue-input",
                ));
            }
        };

        entries.push(IssueEntry {
            id,
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
        });
    }

    accept!((
        IssueEditPageRepl {
            chapter_id,
            page_artwork_id,
            entries
        },
        created_issue_ids
    ))
}

// Validates identities and preserves existing legacy metadata.
async fn preserve_and_validate<C, R>(
    repo: &R,
    context: &mut C,
    replacement: &mut IssueEditPageRepl,
    created_issue_ids: &[IssueSaveCreatedIssueId],
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
        if !created_issue_ids
            .iter()
            .any(|pair| pair.issue_id == entry.id)
            && !existing.iter().any(|info| info.id == entry.id)
        {
            return Err(issue_complex::error(
                ExpectedVariant::Args,
                "error-issue-input",
            ));
        }

        //
        entry.layer_name = existing
            .iter()
            .find(|info| info.id == entry.id)
            .and_then(|info| info.layer_name.clone());
    }

    accept!(())
}

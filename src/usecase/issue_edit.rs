//! Versioned annotation editing independent of delivery mechanisms.

// Chapter write authorization.
mod access;
// Consistent document projections.
mod document;

/// Atomic chapter category rename.
pub mod rename;
/// Idempotent ver-checked page saves.
pub mod save;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;

use crate::data::val::issue_edit::{IssueEditPageVal, IssueEditVariantsVal};
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageArtworkImage;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::issue_edit::IssueEditRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::page_artwork::GetPageArtworkInfo;
use crate::part::repo::page_artwork::PageArtworkRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest};
use crate::usecase::page::list as page_list_usecase;

/// Reads one consistent page document, including its current image generation.
#[instrument(level = "info",
         skip(nucl,
         repo,
         obj_dept,
         token),
         fields(actor_user_id = %token.user_id))]
pub async fn get_info<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    page_id: String,
) -> BaseRest<IssueEditPageVal>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: PageArtworkRepo<C>
        + IssueEditRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync
        + Send,
    O: ObjDeptView<PageArtworkImage, C> + Sync + Send,
{
    let page_info = GetPageArtworkInfo { id: &page_id }.run_on(repo).await?;

    page_list_usecase::ensure_user_can_list_infos::<C, R>(
        repo,
        &token,
        &page_info.chapter_id,
    )
    .await?;

    let output = nucl
        .coord(async move |context| {
            //
            document::page(
                repo,
                obj_dept,
                context,
                &page_info.chapter_id,
                &page_id,
            )
            .await
        })
        .await?;

    accept!(output)
}

/// Lists category usage across every stored page, without local user presets.
#[instrument(level = "info",
         skip(nucl,
         repo,
         token),
         fields(actor_user_id = %token.user_id))]
pub async fn list_variant_infos<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    chapter_id: String,
) -> BaseRest<IssueEditVariantsVal>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: IssueEditRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync
        + Send,
{
    page_list_usecase::ensure_user_can_list_infos::<C, R>(
        repo,
        &token,
        &chapter_id,
    )
    .await?;

    let output = nucl
        .coord(async move |context| {
            document::variants(repo, context, &chapter_id).await
        })
        .await?;

    accept!(output)
}

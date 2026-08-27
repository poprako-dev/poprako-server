//! Page use cases — image reservation, listing, upload confirmation, and deletion.

/// Page deletion use case.
pub mod delete;
/// Page read orchestration.
pub mod list;
/// Page reservation workflow and related orchestration.
pub mod reserve;

#[cfg(test)]
// Unit tests for page metadata and upload reservation flows.
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::page::PagePermComplex;
use crate::data::instr::page::MarkPageImageUploadedInstr;
use crate::model::read::proj::page::PageInfo;
use crate::model::shared::user::UserToken;
use crate::model::write::page::PageImageRepl;
use crate::part::image::ImageManager;
use crate::part::nucl::ReptRead;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::page::{
    GetPageInfo, GetPageInfoExcluded, MarkPageImageUploaded,
};
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Marks one page image as uploaded.
#[instrument(level = "info", skip(nucl, repo, image_manager))]
pub async fn mark_image_uploaded<N, C, R, I>(
    (nucl, repo, image_manager): (&N, &R, &I),
    token: UserToken,
    id: String,
    instr: MarkPageImageUploadedInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C> + PageRepo<C> + AssignmentRepo<C> + Send + Sync,
    I: ImageManager,
{
    let page_info = GetPageInfo { id: &id }.run_on(repo).await?;

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &page_info.chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        let err_message = trl("error-page-upload-role-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %page_info.chapter_id,
            user_id = %token.user_id,
            "expected error: page upload assignment missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    PagePermComplex::ensure_user_can_mark_image_uploaded(&assignment_info)?;

    if page_info.image_version != Some(instr.image_version) {
        //
        return Err(stale_page_image_err(
            &page_info,
            &token.user_id,
            instr.image_version,
            None,
        ));
    }

    if page_info.is_image_uploaded == Some(true) {
        return accept(());
    }

    let image_key = page_info.image_key.clone().ok_or_else(|| {
        //
        stale_page_image_err(
            &page_info,
            &token.user_id,
            instr.image_version,
            None,
        )
    })?;

    if !image_manager.object_exists(&image_key).await? {
        //
        return Err(stale_page_image_err(
            &page_info,
            &token.user_id,
            instr.image_version,
            Some(&image_key),
        ));
    }

    let page_image_repl = PageImageRepl {
        id,
        image_version: instr.image_version,
        image_key: Some(image_key),
        is_image_uploaded: true,
    };

    let () = nucl
        .coord(async move |context| {
            //
            commit_page_image_upload(
                repo,
                context,
                &page_info,
                &page_image_repl,
                &token.user_id,
            )
            .await
        })
        .await?;

    accept(())
}

// Builds and logs the expected error for a stale page-image upload.
fn stale_page_image_err(
    page_info: &PageInfo,
    actor_user_id: &str,
    image_version: u32,
    image_key: Option<&str>,
) -> BaseError {
    //
    let err_message = trl("error-stale-page-image-upload");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        page_id = %page_info.id,
        chapter_id = %page_info.chapter_id,
        user_id = actor_user_id,
        image_version,
        stored_image_version = page_info.image_version,
        image_key,
        "expected error: stale page image upload",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}

// Commits a verified page-image upload inside the page transaction.
async fn commit_page_image_upload<C, R>(
    repo: &R,
    context: &mut C,
    page_info: &PageInfo,
    page_image_repl: &PageImageRepl,
    actor_user_id: &str,
) -> BaseRest<()>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C> + PageRepo<C> + Send + Sync,
{
    // NOTE: Chapter -> Page is the shared lock order that prevents both
    // deadlocks and chapter upload-summary races.
    let chapter_info = GetChapterInfoExcluded {
        id: &page_info.chapter_id,
        incls: &[],
    }
    .step_on(repo, context)
    .await?;

    ChapterComplex::ensure_chapter_writable(&chapter_info)?;

    let locked_page_info = GetPageInfoExcluded {
        id: &page_image_repl.id,
    }
    .step_on(repo, context)
    .await?;

    if locked_page_info.image_version != Some(page_image_repl.image_version)
        || locked_page_info.image_key.as_deref()
            != page_image_repl.image_key.as_deref()
    {
        let err_message = trl("error-stale-page-image-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            page_id = %page_image_repl.id,
            chapter_id = %page_info.chapter_id,
            user_id = actor_user_id,
            image_version = page_image_repl.image_version,
            locked_image_version = locked_page_info.image_version,
            image_key = ?page_image_repl.image_key,
            "expected error: stale page image upload",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    MarkPageImageUploaded {
        repl: page_image_repl,
    }
    .step_on(repo, context)
    .await?;

    accept(())
}

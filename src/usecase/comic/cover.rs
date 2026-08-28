//! Comic cover upload confirmation.

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::comic::{ComicComplex, ComicPermComplex};
use crate::data::instr::comic::MarkComicCoverUploadedInstr;
use crate::model::shared::user::UserToken;
use crate::part::image::ImageManager;
use crate::part::nucl::ReptRead;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comic::{
    GetComicInfo, GetComicInfoExcluded, MarkComicCoverUploaded,
};
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;

// Builds the client-visible error for a stale comic cover upload.
macro_rules! stale_cover_error {
    ($id:expr, $user_id:expr, $image_version:expr, $($field:tt)*) => {{
        let err_message = trl("error-stale-cover-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            comic_id = %$id,
            user_id = %$user_id,
            image_version = $image_version,
            $($field)*
            "expected error: stale comic cover upload",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    }};
}

/// Marks a reserved comic cover as successfully uploaded.
#[instrument(level = "info", skip(nucl, repo, image_manager))]
pub async fn mark_uploaded<N, C, R, I>(
    (nucl, repo, image_manager): (&N, &R, &I),
    token: UserToken,
    id: String,
    instr: MarkComicCoverUploadedInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ComicRepo<C> + TeamRepo<C> + MemberRepo<C> + Send + Sync,
    I: ImageManager,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_mark_cover_uploaded(&member_info)?;

    let comic_info = GetComicInfo {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    ComicComplex::ensure_comic_writable(&comic_info)?;

    if comic_info.cover_version != Some(instr.image_version) {
        //
        return Err(stale_cover_error!(
            &id,
            &token.user_id,
            instr.image_version,
            stored_image_version = comic_info.cover_version,
        ));
    }

    if comic_info.is_cover_uploaded == Some(true) {
        return accept(());
    }

    let cover_key = comic_info.cover_key.clone().ok_or_else(|| {
        //
        stale_cover_error!(
            &id,
            &token.user_id,
            instr.image_version,
            stored_image_version = comic_info.cover_version,
        )
    })?;

    ensure_cover_object_exists(
        image_manager,
        (&id, &token.user_id, &cover_key),
        instr.image_version,
    )
    .await?;

    nucl.coord(async move |context| {
        //
        let locked_comic_info = GetComicInfoExcluded {
            id: &id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        ComicComplex::ensure_comic_writable(&locked_comic_info)?;

        if locked_comic_info.cover_version != Some(instr.image_version)
            || locked_comic_info.cover_key.as_deref() != Some(&cover_key)
        {
            return Err(stale_cover_error!(
                &id,
                &token.user_id,
                instr.image_version,
                locked_image_version = locked_comic_info.cover_version,
                cover_key = %cover_key,
            ));
        }

        MarkComicCoverUploaded {
            id: &id,
            cover_version: instr.image_version,
            cover_key: Some(&cover_key),
            cover_uploaded: true,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

// Confirms that the pending cover object exists in image storage.
async fn ensure_cover_object_exists<I>(
    image_manager: &I,
    (id, user_id, cover_key): (&str, &str, &str),
    image_version: u32,
) -> BaseRest<()>
where
    I: ImageManager,
{
    if image_manager.object_exists(cover_key).await? {
        return accept(());
    }

    Err(stale_cover_error!(
        id,
        user_id,
        image_version,
        cover_key = %cover_key,
    ))
}

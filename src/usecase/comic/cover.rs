//! Comic-cover optimistic upload availability.

use poprako_orchestra::{Context, OperRun as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::key::ObjGen;
use poprako_obj_dept::oper::MarkObjUploaded;
use poprako_util::i18n::trl;

use crate::complex::comic::ComicPermComplex;
use crate::data::instr::comic::MarkComicCoverUploadedInstr;
use crate::model::shared::user::UserToken;
use crate::part::obj_dept::ComicCover;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;

/// Optimistically marks the requested current cover generation as uploaded.
#[instrument(level = "info", skip(repo, obj_dept))]
pub async fn mark_uploaded<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    id: String,
    instr: MarkComicCoverUploadedInstr,
) -> BaseRest<()>
where
    C: Context,
    R: ComicRepo<C> + TeamRepo<C> + MemberRepo<C> + Sync,
    O: ObjDept<ComicCover, C> + Sync,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_mark_cover_uploaded(&member_info)?;

    // SAFETY: This is an optimistic exact-generation transition. It does not
    // synchronously prove PUT success, object presence, or content integrity;
    // the delayed actor may reset this generation after a failed HEAD check.
    let cover_key = ObjGen {
        id,
        ver: instr.image_ver,
    };

    let marked = MarkObjUploaded::<ComicCover>::new(&cover_key)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    marked.then_some(()).ok_or_else(|| BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-stale-cover-upload"),
    })
}

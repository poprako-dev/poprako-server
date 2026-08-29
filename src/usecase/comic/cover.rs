//! Comic-cover verification status.

use poprako_orchestra::{Context, OperRun as _, Run};
use tracing::instrument;

use poprako_obj_dept::obj_inst;
use poprako_obj_dept::oper::GetObjMeta;
use poprako_obj_dept::rest::ObjDeptError;
use poprako_util::i18n::trl;

use crate::complex::comic::ComicPermComplex;
use crate::data::instr::comic::MarkComicCoverUploadedInstr;
use crate::model::shared::user::UserToken;
use crate::part::obj_dept::ComicCover;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;

/// Confirms the requested cover generation is the current `ObjDept` object.
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
    O: for<'a> Run<GetObjMeta<'a, ComicCover>, Error = ObjDeptError> + Sync,
{
    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_mark_cover_uploaded(&member_info)?;

    let obj_meta = obj_inst! { GetObjMeta<ComicCover> { id: &id } }
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    match obj_meta {
        //
        Some(obj_meta) if obj_meta.key.version == instr.image_version => {
            accept(())
        }

        _ => Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-stale-cover-upload"),
        }),
    }
}

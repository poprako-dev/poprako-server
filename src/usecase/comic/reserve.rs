use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::model::slot::ObjSlotSpec;
use poprako_obj_dept::oper::GenObjSlot;

use crate::complex::comic::{ComicComplex, ComicPermComplex};
use crate::complex::image::ImageComplex;
use crate::config::image::ImageConfig;
use crate::data::instr::comic::ReserveComicCoverInstr;
use crate::data::val::comic::ReserveComicCoverVal;
use crate::data::view::image::ImageUploadSlotView;
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::ComicCover;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comic::GetComicInfoExcluded;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;
use crate::value::image::{ComicCoverKey, ImageKind};

/// Reserves a new comic cover upload slot.
#[instrument(level = "info", skip(nucl, repo, obj_dept, image_config))]
pub async fn reserve_cover<N, C, R, O>(
    (nucl, repo, obj_dept, image_config): (&N, &R, &O, &ImageConfig),
    token: UserToken,
    id: String,
    instr: ReserveComicCoverInstr,
) -> BaseRest<ReserveComicCoverVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ComicRepo<C> + TeamRepo<C> + MemberRepo<C> + Send + Sync,
    O: ObjDept<ComicCover, C> + Send + Sync,
{
    ImageComplex::ensure_byte_length(
        image_config,
        instr.new_byte_len,
        ImageKind::ComicCover,
    )?;

    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_reserve_cover(&member_info)?;

    let obj_slot = nucl
        .coord(async move |context| {
            //
            let comic_info = GetComicInfoExcluded {
                id: &id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ComicComplex::ensure_comic_writable(&comic_info)?;

            let obj_spec = ObjSlotSpec {
                dom: ComicCoverKey {
                    comic_id: id.clone(),
                    ext: instr.ext,
                },
                hash: instr.image_hash.as_bytes(),
                content_type: instr.ext.content_type(),
                byte_len: instr.new_byte_len,
            };

            GenObjSlot::<ComicCover>::new(&obj_spec)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)
        })
        .await?;

    let slot = Some(ImageUploadSlotView {
        put_url: obj_slot.url.to_string(),
        image_version: obj_slot.key.version,
        headers: obj_slot.headers,
    });

    accept(ReserveComicCoverVal { slot })
}

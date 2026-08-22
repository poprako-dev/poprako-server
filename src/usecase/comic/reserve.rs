use std::time::Duration;

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use crate::complex::comic::{ComicComplex, ComicPermComplex};
use crate::complex::image::ImageComplex;
use crate::config::ImageConfig;
use crate::data::instr::comic::ReserveComicCoverInstr;
use crate::data::val::comic::ReserveComicCoverVal;
use crate::data::view::image::ImageUploadSlotView;
use crate::model::shared::user::UserToken;
use crate::part::image::{ImagePool, ImageUploadSpec};
use crate::part::nucl::ReptRead;
use crate::part::prom::Prom;
use crate::part::prom::oper::DeferBatch;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::prom::task::Task;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comic::{GetComicInfoExcluded, ReserveComicCover};
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;
use crate::value::image::ImageKind;

/// Reserves a new comic cover upload slot.
#[instrument(level = "info", skip(nucl, repo, prom, image_pool, image_config))]
pub async fn reserve_cover<N, C, R, P, I>(
    (nucl, repo, prom, image_pool, image_config): (
        &N,
        &R,
        &P,
        &I,
        &ImageConfig,
    ),
    token: UserToken,
    id: String,
    instr: ReserveComicCoverInstr,
) -> BaseRest<ReserveComicCoverVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<ReptRead>,
    R: ComicRepo<C> + TeamRepo<C> + MemberRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    ImageComplex::ensure_byte_length(
        image_config,
        instr.new_byte_len,
        ImageKind::ComicCover,
    )?;

    let (transaction_image_hash, image_ext, new_byte_len) =
        (instr.image_hash, instr.ext, instr.new_byte_len);

    let member_info = MemberLoader::load_info_from_comic(
        repo,
        LoadMode::Run,
        &token.user_id,
        &id,
    )
    .await?;

    ComicPermComplex::ensure_user_can_reserve_cover(&member_info)?;

    let (object_key, cover_version, upload_required) = nucl
        .coord(async move |context| {
            //
            let comic_info = GetComicInfoExcluded {
                id: &id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ComicComplex::ensure_comic_writable(&comic_info)?;

            let cover_reservation = ReserveComicCover {
                id: &id,
                image_hash: &transaction_image_hash,
                image_ext,
            }
            .step_on(repo, context)
            .await?;

            if !cover_reservation.is_upload_required {
                //
                return accept((
                    cover_reservation.object_key,
                    cover_reservation.cover_version,
                    false,
                ));
            }

            let (mut batch_ids, mut batch_payloads, mut batch_delays) =
                (Vec::new(), Vec::new(), Vec::new());

            if let Some(prev_object_key) = &cover_reservation.prev_object_key {
                //
                batch_ids.push(ImageComplex::gen_delete_id());

                batch_payloads.push(TaskPayload::Image {
                    payload: image::ImagePayload::Delete {
                        object_key: prev_object_key.clone(),
                    },
                });

                batch_delays.push(None);
            }

            batch_ids.push(ImageComplex::gen_check_id());

            batch_payloads.push(TaskPayload::Image {
                payload: image::ImagePayload::CheckUpload {
                    image_kind: ImageKind::ComicCover,
                    resource_id: id.clone(),
                    object_key: cover_reservation.object_key.clone(),
                    version: cover_reservation.cover_version,
                },
            });

            batch_delays.push(Some(Duration::from_secs(15 * 60)));

            let batch_tasks = batch_ids
                .iter()
                .zip(batch_payloads.iter())
                .zip(batch_delays.iter())
                .map(|((id, payload), delay)| Task {
                    id,
                    payload,
                    delay: *delay,
                })
                .collect::<Vec<Task<'_, String, TaskPayload>>>();

            DeferBatch::new(&batch_tasks).step_on(prom, context).await?;

            accept((
                cover_reservation.object_key,
                cover_reservation.cover_version,
                true,
            ))
        })
        .await?;

    let slot = match upload_required {
        //
        true => {
            //
            let upload_spec = ImageUploadSpec {
                object_key: &object_key,
                content_type: image_ext.content_type(),
                content_length: new_byte_len,
            };

            let upload_slot = image_pool.get_upload_slot(upload_spec).await?;

            Some(ImageUploadSlotView {
                put_url: upload_slot.url.to_string(),
                image_version: cover_version,
                headers: upload_slot.headers,
            })
        }

        false => None,
    };

    accept(ReserveComicCoverVal { slot })
}

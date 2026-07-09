//! Comic use cases — create, read, update, cover management, and deletion.

use std::collections::HashMap;

use time::{Duration, OffsetDateTime};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::ChapterComplex;
use crate::complex::comic::{ComicComplex, ComicPermComplex};
use crate::complex::image::ImageComplex;
use crate::data::chapter::ChapterInfoVal;
use crate::data::comic::{
    ComicInfoVal, CreateComicData, CreateComicVal, ListComicInfosData,
    MarkComicCoverUploadedData, ReserveComicCoverData, ReserveComicCoverVal,
    UpdateComicInfoData,
};
use crate::model::assignment::AssignmentForm;
use crate::model::chapter::ChapterForm;
use crate::model::comic::{ComicForm, ComicInfoUpdate, ComicListSpec};
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::prom::task::{IMAGE_TOPIC, ImageKind, ImageTask};
use crate::part::prom::{Payload, Prom, PromStep};
use crate::part::repo::assignment::{
    AssignmentRepo, AssignmentRepoTransactional,
};
use crate::part::repo::assignment_invitation::{
    AssignmentInvitationRepo, AssignmentInvitationRepoTransactional,
};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::step::assignment::AssignmentStep;
use crate::part::repo::step::chapter::ChapterStep;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::workset::WorksetStep;
use crate::part::repo::unit::{UnitRepo, UnitRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{RegularError, RegularResult};
use crate::util::DeriveTransactional;
use crate::value::comic::ComicWithOpt;
use crate::value::role::{RoleField, RoleMask};

#[cfg(test)]
pub mod tests;

/// Creates a new comic inside a workset together with its first
/// chapter and a creator admin assignment.
///
/// Inside a single transaction this:
/// 1. Allocates a workset-scoped comic index.
/// 2. Inserts the comic row.
/// 3. Bumps the workset comic count.
/// 4. Allocates a chapter index and inserts the first (pinned) chapter.
/// 5. Updates the comic's denormalised chapter counter and last-activity
///    timestamp.
/// 6. Creates an ADMIN assignment on the new chapter for the caller.
pub async fn create<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: CreateComicData,
) -> RegularResult<CreateComicVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + AssignmentRepoTransactional<C>
        + Send
        + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ComicPermComplex::can_user_create(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.workset_id,
    )
    .await?;

    let (comic_id, chapter_id) = drive
        .with_context(async move |context| -> RegularResult<(String, String)> {
            //
            let repo = repo.derive_transactional().await;

            let index = repo
                .advance(
                    context,
                    &WorksetStep::incr_comic_next_index(&data.workset_id),
                )
                .await?;

            let comic_form = ComicForm {
                id: ComicComplex::gen_id(),
                workset_id: data.workset_id,
                index,
                title: data.title,
                author: data.author,
                description: data.description,
                creator_id: token.user_id.clone(),
            };

            let comic_info = repo
                .advance(context, &ComicStep::create(&comic_form))
                .await?;

            repo.advance(
                context,
                &WorksetStep::update_comic_count(&comic_form.workset_id, 1),
            )
            .await?;

            let chapter_index = repo
                .advance(
                    context,
                    &ComicStep::incr_chapter_next_index(&comic_info.id),
                )
                .await?;

            let subtitle = ChapterComplex::subtitle_or_default(
                data.first_chapter_subtitle,
                chapter_index,
            );

            let chapter_form = ChapterForm {
                id: ChapterComplex::gen_id(),
                comic_id: comic_info.id.clone(),
                is_pinned: true,
                index: chapter_index,
                subtitle,
                creator_id: token.user_id.clone(),
            };

            let chapter_info = repo
                .advance(context, &ChapterStep::create(&chapter_form))
                .await?;

            repo.advance(
                context,
                &ChapterStep::unpin_others(
                    &chapter_info.comic_id,
                    &chapter_info.id,
                ),
            )
            .await?;

            repo.advance(
                context,
                &ComicStep::update_chapter_count(&chapter_info.comic_id, 1),
            )
            .await?;

            repo.advance(
                context,
                &ComicStep::touch_last_active(&chapter_info.comic_id),
            )
            .await?;

            let assignment_form = AssignmentForm {
                id: AssignmentComplex::gen_id(),
                chapter_id: chapter_info.id.clone(),
                user_id: token.user_id,
                roles: RoleMask::from(RoleField::ADMIN),
            };

            repo.advance(context, &AssignmentStep::create(&assignment_form))
                .await?;

            Ok((comic_info.id, chapter_info.id))
        })
        .await?;

    Ok(CreateComicVal {
        id: comic_id,
        chapter_id,
    })
}

/// Fetches a comic by ID with cover URL resolution.
pub async fn get_info<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    id: String,
) -> RegularResult<ComicInfoVal>
where
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>,
    I: ImagePool,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ComicPermComplex::can_user_get_info(
        &mut repo.as_proxy(),
        &token.user_id,
        &id,
    )
    .await?;

    let comic_info = repo.execute(&ComicStep::get_info_by_id(&id, &[])).await?;

    ComicInfoVal::from_model(image_pool, comic_info, None).await
}

/// Lists comics for a workset with optional title filter, completion filter, and pagination.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    data: ListComicInfosData,
) -> RegularResult<Vec<ComicInfoVal>>
where
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + ChapterRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + ChapterRepoTransactional<C>,
    I: ImagePool,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ComicPermComplex::can_user_list_infos(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.workset_id,
    )
    .await?;

    let with_pinned_chapter =
        data.with_opt.contains(&ComicWithOpt::PinnedChapter);

    let spec: ComicListSpec = data.try_into()?;

    let comic_infos = repo.execute(&ComicStep::list_infos(&spec)).await?;

    // NOTE: `with` cannot be executed elegantly by repo layer,
    // so we have to handle it in usecase layer.
    let mut pinned_chapters = if with_pinned_chapter {
        let comic_ids: Vec<String> =
            comic_infos.iter().map(|info| info.id.clone()).collect();

        repo.execute(&ChapterStep::list_pinned_infos_by_comic_ids(&comic_ids))
            .await?
    } else {
        HashMap::new()
    };

    let mut comic_info_vals = Vec::with_capacity(comic_infos.len());

    for comic_info in comic_infos {
        // FIXME: spacing.
        let pinned_chapter_val = match pinned_chapters.remove(&comic_info.id) {
            Some(chapter_info) => Some(
                ChapterInfoVal::from_model(image_pool, chapter_info).await?,
            ),
            None => None,
        };

        comic_info_vals.push(
            ComicInfoVal::from_model(
                image_pool,
                comic_info,
                pinned_chapter_val,
            )
            .await?,
        );
    }

    Ok(comic_info_vals)
}

/// Updates a comic's title, author, and description.
pub async fn update_info<C, R>(
    repo: &R,
    token: UserToken,
    data: UpdateComicInfoData,
) -> RegularResult<()>
where
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ComicPermComplex::can_user_update_info(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.id,
    )
    .await?;

    let comic_info_update = ComicInfoUpdate {
        id: data.id,
        title: data.title,
        author: data.author,
        description: data.description,
    };

    repo.execute(&ComicStep::update_info(&comic_info_update))
        .await?;

    Ok(())
}

/// Reserves a new comic cover upload slot.
pub async fn reserve_cover<D, C, R, P, I>(
    drive: &D,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    id: String,
    data: ReserveComicCoverData,
) -> RegularResult<ReserveComicCoverVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ComicPermComplex::can_user_reserve_cover(
        &mut repo.as_proxy(),
        &token.user_id,
        &id,
    )
    .await?;

    let (object_key, cover_version) = drive
        .with_context(async move |context| -> RegularResult<(String, i64)> {
            //
            let repo = repo.derive_transactional().await;

            let cover_reservation = repo
                .advance(
                    context,
                    &ComicStep::reserve_cover(&id, &data.file_ext),
                )
                .await?;

            let now = OffsetDateTime::now_utc();

            if let Some(prev_key) = &cover_reservation.prev_object_key {
                //
                let delete_id = ImageComplex::gen_delete_id();

                prom.advance(
                    context,
                    &PromStep::append(
                        &delete_id,
                        IMAGE_TOPIC,
                        Payload::Image(ImageTask::Delete {
                            object_key: prev_key.as_str(),
                        }),
                        &now,
                    ),
                )
                .await?;
            }

            let check_id = ImageComplex::gen_check_id();

            let check_visible_at = now + Duration::minutes(15);

            prom.advance(
                context,
                &PromStep::append(
                    &check_id,
                    IMAGE_TOPIC,
                    Payload::Image(ImageTask::CheckUploaded {
                        kind: ImageKind::ComicCover,
                        resource_id: &id,
                        object_key: &cover_reservation.object_key,
                        image_version: cover_reservation.cover_version,
                    }),
                    &check_visible_at,
                ),
            )
            .await?;

            Ok((
                cover_reservation.object_key,
                cover_reservation.cover_version,
            ))
        })
        // FIXME: spacing
        .await?;

    let put_url = image_pool.put_signed(&object_key).await?.to_string();

    Ok(ReserveComicCoverVal {
        put_url,
        cover_version,
    })
}

/// Marks a reserved comic cover as successfully uploaded.
pub async fn mark_cover_uploaded<C, R>(
    repo: &R,
    token: UserToken,
    id: String,
    data: MarkComicCoverUploadedData,
) -> RegularResult<()>
where
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ComicPermComplex::can_user_mark_cover_uploaded(
        &mut repo.as_proxy(),
        &token.user_id,
        &id,
    )
    .await?;

    repo.execute(&ComicStep::mark_cover_uploaded(&id, data.cover_version))
        .await?;

    Ok(())
}

/// Deletes a comic and updates the parent workset counter.
pub async fn delete<D, C, R, P>(
    drive: &D,
    repo: &R,
    prom: &P,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + PageRepo<C>
        + AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
    <R as DeriveTransactional>::Transactional:
        ComicRepoTransactional<C>
            + WorksetRepoTransactional<C>
            + MemberRepoTransactional<C>
            + ChapterRepoTransactional<C>
            + PageRepoTransactional<C>
            + AssignmentInvitationRepoTransactional<C>
            + AssignmentRepoTransactional<C>
            + UnitRepoTransactional<C>
            + Send
            + Sync,
    P: Prom<C> + Send + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ComicPermComplex::can_user_delete(
        &mut repo.as_proxy(),
        &token.user_id,
        &id,
    )
    .await?;

    drive
        .with_context(async move |context| -> RegularResult<()> {
            //
            let repo = repo.derive_transactional().await;

            let comic_info = repo
                .advance(context, &ComicStep::get_info_excluded(&id, &[]))
                .await?;

            ComicComplex::delete_cascade(&repo, prom, context, &comic_info.id)
                .await?;

            Ok(())
        })
        .await?;

    Ok(())
}

/// Marks a comic archived.
///
/// TODO: Archiving is not fully implemented yet. This currently only marks
/// the comic with the existing completed flag and does not run a cascade
/// archive/delete workflow.
pub async fn mark_archived<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + Send
        + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ComicPermComplex::can_user_mark_archived(
        &mut repo.as_proxy(),
        &token.user_id,
        &id,
    )
    .await?;

    drive
        .with_context(async move |context| -> RegularResult<()> {
            //
            let repo = repo.derive_transactional().await;

            repo.advance(context, &ComicStep::mark_archived(&id))
                .await?;

            Ok(())
        })
        .await?;

    Ok(())
}

//! RDB-backed comic stage filtering.

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;

use poprako_rdb_core::RdbConn;

use crate::part_impl::repo::rdb_impl::schema::t_chapter::dsl::{
    f_comic_id as chapter_comic_id, f_deleted_at as chapter_deleted_at,
    f_is_pinned as chapter_is_pinned, f_proofread_at as chapter_proofread_at,
    f_proofreading_at as chapter_proofreading_at,
    f_published_at as chapter_published_at,
    f_reviewed_at as chapter_reviewed_at,
    f_translated_at as chapter_translated_at,
    f_translating_at as chapter_translating_at,
    f_typeset_at as chapter_typeset_at,
    f_typesetting_at as chapter_typesetting_at,
    f_uploaded_at as chapter_uploaded_at, t_chapter,
};
use crate::result::{BaseRest, accept};
use crate::shared::result::diesel;
use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::{Stage, StagePhase};

/// List comic IDs with chapters in any stage not ignored by the mask.
pub async fn list_matching_stage_comic_ids(
    conn: &mut RdbConn,
    stage_mask: StageMask,
) -> BaseRest<Option<Vec<String>>> {
    //
    let stages = StageMask::stages()
        .iter()
        .copied()
        .filter(|stage| !stage_mask.ignores_stage(*stage))
        .collect::<Vec<_>>();

    if stages.is_empty() {
        return accept(None);
    }

    let mut query = t_chapter
        .filter(chapter_deleted_at.is_null())
        .filter(chapter_is_pinned.eq(true))
        .select(chapter_comic_id)
        .distinct()
        .into_boxed();

    for stage in stages {
        //
        let phase = stage_mask.get_phase(stage);

        query = match (stage, phase) {
            //
            (Stage::RawProvide, StagePhase::Pending) => {
                query.filter(chapter_uploaded_at.is_null())
            }

            (Stage::RawProvide, StagePhase::Completed) => {
                query.filter(chapter_uploaded_at.is_not_null())
            }

            (Stage::Translate, StagePhase::Pending) => query
                .filter(chapter_translating_at.is_null())
                .filter(chapter_translated_at.is_null()),

            (Stage::Translate, StagePhase::Active) => query
                .filter(chapter_translating_at.is_not_null())
                .filter(chapter_translated_at.is_null()),

            (Stage::Translate, StagePhase::Completed) => {
                query.filter(chapter_translated_at.is_not_null())
            }

            (Stage::Proofread, StagePhase::Pending) => query
                .filter(chapter_proofreading_at.is_null())
                .filter(chapter_proofread_at.is_null()),

            (Stage::Proofread, StagePhase::Active) => query
                .filter(chapter_proofreading_at.is_not_null())
                .filter(chapter_proofread_at.is_null()),

            (Stage::Proofread, StagePhase::Completed) => {
                query.filter(chapter_proofread_at.is_not_null())
            }

            (Stage::TypesetRedraw, StagePhase::Pending) => query
                .filter(chapter_typesetting_at.is_null())
                .filter(chapter_typeset_at.is_null()),

            (Stage::TypesetRedraw, StagePhase::Active) => query
                .filter(chapter_typesetting_at.is_not_null())
                .filter(chapter_typeset_at.is_null()),

            (Stage::TypesetRedraw, StagePhase::Completed) => {
                query.filter(chapter_typeset_at.is_not_null())
            }

            (Stage::Review, StagePhase::Pending) => {
                query.filter(chapter_reviewed_at.is_null())
            }

            (Stage::Review, StagePhase::Completed) => {
                query.filter(chapter_reviewed_at.is_not_null())
            }

            (Stage::Publish, StagePhase::Pending) => {
                query.filter(chapter_published_at.is_null())
            }

            (Stage::Publish, StagePhase::Completed) => {
                query.filter(chapter_published_at.is_not_null())
            }

            (
                Stage::RawProvide | Stage::Review | Stage::Publish,
                StagePhase::Active,
            ) => return accept(Some(Vec::new())),
        };
    }

    let comic_ids = query.load(conn).await.map_err(diesel)?;

    accept(Some(comic_ids))
}

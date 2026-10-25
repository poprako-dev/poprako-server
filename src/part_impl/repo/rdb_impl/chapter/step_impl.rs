//! RDB-backed chapter repository step implementations.

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::chapter::{ChapterInfo, ChapterUnitEditScope};
use crate::model::read::proj::unit::UnitCountDelta;
use crate::model::read::spec::chapter::ChapterListSpec;
use crate::model::write::chapter::{ChapterPatch, ChapterStageRepl};
use crate::part::repo::oper::chapter::SetChapterPageSourceCountMetrics;
use crate::part_impl::repo::rdb_impl::entity::chapter::{
    ChapterInfoRecord, ChapterItem, ChapterPack, ChapterUnitEditScopeRecord,
};
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::numeric::i32_from_usize;
use crate::part_impl::repo::rdb_impl::schema::t_chapter::dsl::{
    f_comic_id, f_deleted_at, f_id, f_index, f_is_pinned, f_page_source_count,
    f_proofread_at, f_proofread_unit_count, f_proofreading_at, f_published_at,
    f_total_unit_count, f_translated_at, f_translated_unit_count,
    f_translating_at, f_typeset_at, f_typesetting_at, f_updated_at,
    f_uploaded_at, t_chapter,
};
use crate::part_impl::repo::rdb_impl::schema::t_page_source;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::shape::write::chapter::ChapterEntryShape;
use crate::shared::result::diesel;
use crate::value::chapter::ChapterInclOpt;
use crate::value::chapter::stage::Stage;

/// Build the expected error for a missing chapter.
pub fn missing_chapter(id: &str, operation: &str) -> BaseError {
    //
    expected_error! {
        ExpectedVariant::Args,
        message: trl("error-chapter-not-found");
        chapter_id = %id,
        operation,
        "chapter not found",
    }
}

/// Queries a single chapter record by ID and populates its includes.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> BaseRest<ChapterInfo> {
    //
    let record = t_chapter
        .filter(f_id.eq(id))
        .filter(f_deleted_at.is_null())
        .select(ChapterInfoRecord::as_select())
        .get_result::<ChapterInfoRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            expected_error! {
                ExpectedVariant::Args,
                message: trl("error-chapter-not-found");
                error_variant = ?ExpectedVariant::Args,
                chapter_id = %id,
                stage = "get_info_by_id",
                "chapter not found",
            }
        })?;

    let mut info = record_into_info(record)?;

    incl::chapter::populate_chapter_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    accept!(info)
}

/// Queries a single chapter record by ID under `FOR UPDATE` lock.
#[instrument(level = "info", skip_all)]
pub async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> BaseRest<ChapterInfo> {
    //
    let record = t_chapter
        .filter(f_id.eq(id))
        .filter(f_deleted_at.is_null())
        .select(ChapterInfoRecord::as_select())
        .for_update()
        .get_result::<ChapterInfoRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            expected_error! {
                ExpectedVariant::Args,
                message: trl("error-chapter-not-found");
                error_variant = ?ExpectedVariant::Args,
                chapter_id = %id,
                stage = "get_info_excluded",
                "chapter not found",
            }
        })?;

    let mut info = record_into_info(record)?;

    incl::chapter::populate_chapter_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    accept!(info)
}

/// Locks the minimal Chapter edit scope owning the requested source page.
#[instrument(level = "info", skip_all)]
pub async fn get_unit_edit_scope_excluded(
    conn: &mut RdbConn,
    page_source_id: &str,
) -> BaseRest<ChapterUnitEditScope> {
    //
    let record = t_chapter
        .filter(
            f_id.eq_any(
                t_page_source::table
                    .filter(t_page_source::f_id.eq(page_source_id))
                    .select(t_page_source::f_chapter_id),
            ),
        )
        .filter(f_deleted_at.is_null())
        .select(ChapterUnitEditScopeRecord::as_select())
        .for_update()
        .get_result::<ChapterUnitEditScopeRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            expected_error! {
                ExpectedVariant::Args,
                message: trl("error-page-not-found");
                page_source_id,
                operation = "get_unit_edit_scope_excluded",
                "page not found",
            }
        })?;

    accept!(record.into())
}

/// Queries chapter records for a given comic, ordered by index descending.
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    conn: &mut RdbConn,
    spec: &ChapterListSpec,
) -> BaseRest<Vec<ChapterInfo>> {
    //
    let records = t_chapter
        .filter(f_comic_id.eq(spec.comic_id.as_str()))
        .filter(f_deleted_at.is_null())
        .select(ChapterInfoRecord::as_select())
        .order_by(f_index.desc())
        .offset(i64::from(spec.offset))
        .limit(i64::from(spec.limit.get()))
        .load::<ChapterInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    let mut infos = records_into_infos(records)?;

    incl::chapter::populate_chapter_incls(conn, &mut infos, &spec.incl_opt)
        .await?;

    accept!(infos)
}

/// Queries all chapter records for a comic under `FOR UPDATE` lock.
#[instrument(level = "info", skip_all)]
pub async fn list_infos_excluded(
    conn: &mut RdbConn,
    comic_id: &str,
) -> BaseRest<Vec<ChapterInfo>> {
    //
    let records = t_chapter
        .filter(f_comic_id.eq(comic_id))
        .filter(f_deleted_at.is_null())
        .select(ChapterInfoRecord::as_select())
        .order_by(f_index.desc())
        .for_update()
        .load::<ChapterInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    records_into_infos(records)
}

/// Locks all chapter records belonging to a comic.
#[instrument(level = "info", skip_all)]
pub async fn lock_chapters(conn: &mut RdbConn, comic_id: &str) -> BaseRest<()> {
    //
    let _ = t_chapter
        .filter(f_comic_id.eq(comic_id))
        .filter(f_deleted_at.is_null())
        .select(f_id)
        .for_update()
        .load::<String>(conn)
        .await
        .map_err(diesel)?;

    accept!(())
}

/// Finds the pinned chapter for a given comic ID, if one exists.
#[instrument(level = "info", skip_all)]
pub async fn find_pinned_info_by_comic_id(
    conn: &mut RdbConn,
    comic_id: &str,
    incl_opt: &[ChapterInclOpt],
) -> BaseRest<Option<ChapterInfo>> {
    //
    let record = t_chapter
        .filter(f_comic_id.eq(comic_id))
        .filter(f_deleted_at.is_null())
        .filter(f_is_pinned.eq(true))
        .select(ChapterInfoRecord::as_select())
        .get_result::<ChapterInfoRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(record) = record else {
        return accept!(None);
    };

    let mut info = record_into_info(record)?;

    incl::chapter::populate_chapter_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    accept!(Some(info))
}

/// Returns the pinned chapter infos for the given comic IDs.
#[instrument(level = "info", skip_all)]
pub async fn list_pinned_infos_by_comic_ids(
    conn: &mut RdbConn,
    comic_ids: &[&str],
) -> BaseRest<Vec<ChapterInfo>> {
    //
    if comic_ids.is_empty() {
        return accept!(Vec::new());
    }

    let records = t_chapter
        .filter(f_comic_id.eq_any(comic_ids))
        .filter(f_deleted_at.is_null())
        .filter(f_is_pinned.eq(true))
        .select(ChapterInfoRecord::as_select())
        .load::<ChapterInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    records.into_iter().map(record_into_info).collect()
}

/// Inserts a new chapter record from the given chapter entry and returns the created info.
#[instrument(level = "info", skip_all)]
pub async fn create(
    conn: &mut RdbConn,
    entry: &ChapterEntryShape<'_>,
) -> BaseRest<ChapterInfo> {
    //
    let item = ChapterItem::try_from(entry)?;

    let record = diesel::insert_into(t_chapter)
        .values(&item)
        .returning(ChapterInfoRecord::as_returning())
        .get_result::<ChapterInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    record_into_info(record)
}

/// Updates the modifiable fields of a chapter record.
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    conn: &mut RdbConn,
    update: &ChapterPatch,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let mut pack = ChapterPack::new(now);

    if let Some(subtitle) = &update.trimmed_subtitle() {
        pack = pack.subtitle(subtitle);
    }

    if let Some(pin) = update.pin {
        pack = pack.pinned(pin);
    }

    let updated_count = diesel::update(
        t_chapter
            .filter(f_id.eq(update.id.as_str()))
            .filter(f_deleted_at.is_null()),
    )
    .set(&pack)
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_chapter(&update.id, "update chapter info"));
    }

    accept!(())
}

/// Updates the stage timestamps of a chapter record.
#[instrument(level = "info", skip_all)]
pub async fn update_stage(
    conn: &mut RdbConn,
    update: &ChapterStageRepl,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let pack = ChapterPack::new(now).stages(update.stages, now);

    let updated_count = diesel::update(
        t_chapter
            .filter(f_id.eq(update.id.as_str()))
            .filter(f_deleted_at.is_null()),
    )
    .set(&pack)
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_chapter(&update.id, "update chapter stage"));
    }

    accept!(())
}

/// Atomically moves a pending two-step stage to active.
#[instrument(level = "info", skip_all)]
pub async fn start_stage(
    conn: &mut RdbConn,
    id: &str,
    stage: Stage,
) -> BaseRest<bool> {
    //
    let now = OffsetDateTime::now_utc();

    let updated_count = match stage {
        //
        Stage::Translate => diesel::update(
            t_chapter
                .filter(f_id.eq(id))
                .filter(f_deleted_at.is_null())
                .filter(f_published_at.is_null())
                .filter(f_translating_at.is_null())
                .filter(f_translated_at.is_null()),
        )
        .set((f_translating_at.eq(now), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?,

        Stage::Proofread => diesel::update(
            t_chapter
                .filter(f_id.eq(id))
                .filter(f_deleted_at.is_null())
                .filter(f_published_at.is_null())
                .filter(f_proofreading_at.is_null())
                .filter(f_proofread_at.is_null()),
        )
        .set((f_proofreading_at.eq(now), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?,

        Stage::TypesetRedraw => diesel::update(
            t_chapter
                .filter(f_id.eq(id))
                .filter(f_deleted_at.is_null())
                .filter(f_published_at.is_null())
                .filter(f_typesetting_at.is_null())
                .filter(f_typeset_at.is_null()),
        )
        .set((f_typesetting_at.eq(now), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?,

        Stage::RawProvide | Stage::Review | Stage::Publish => {
            //
            return Err(unrecoverable_error! {
                message:
                    "only two-step chapter stages can be started".to_string();
                chapter_id = %id,
                stage = ?stage,
                "only two-step chapter stages can be started",
            });
        }
    };

    accept!(updated_count > 0)
}

/// Atomically resolves raw provision when every reserved page is uploaded.
///
/// Missing and already completed chapters are resolved idempotently. A
/// pending chapter returns `false` while at least one upload is incomplete.
#[instrument(level = "info", skip_all)]
pub async fn complete_raw_provide(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<bool> {
    //
    let now = OffsetDateTime::now_utc();

    let updated_count = diesel::update(
        t_chapter
            .filter(f_id.eq(id))
            .filter(f_deleted_at.is_null())
            .filter(f_uploaded_at.is_null())
            .filter(f_page_source_count.gt(0)),
    )
    .set((f_uploaded_at.eq(now), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept!(updated_count > 0)
}

/// Sets the page and unit counters on a chapter record.
#[instrument(level = "info", skip_all)]
pub async fn set_page_source_counts(
    conn: &mut RdbConn,
    oper: &SetChapterPageSourceCountMetrics<'_>,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let pack = ChapterPack::new(now)
        .page_source_count(i32_from_usize(
            oper.page_source_count,
            "t_chapter.f_page_source_count",
        )?)
        .total_unit_count(i32_from_usize(
            oper.total_unit_count,
            "t_chapter.f_total_unit_count",
        )?)
        .translated_unit_count(i32_from_usize(
            oper.translated_unit_count,
            "t_chapter.f_translated_unit_count",
        )?)
        .proofread_unit_count(i32_from_usize(
            oper.proofread_unit_count,
            "t_chapter.f_proofread_unit_count",
        )?);

    let updated_count = diesel::update(
        t_chapter
            .filter(f_id.eq(oper.id))
            .filter(f_deleted_at.is_null()),
    )
    .set(&pack)
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_chapter(oper.id, "set chapter page counters"));
    }

    accept!(())
}

/// Adjusts a chapter's unit counters by the given delta.
#[instrument(level = "info", skip_all)]
pub async fn adjust_unit_counts(
    conn: &mut RdbConn,
    id: &str,
    delta: &UnitCountDelta,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let updated_count = diesel::update(
        t_chapter.filter(f_id.eq(id)).filter(f_deleted_at.is_null()),
    )
    .set((
        f_total_unit_count.eq(f_total_unit_count + delta.total),
        f_translated_unit_count.eq(f_translated_unit_count + delta.translated),
        f_proofread_unit_count.eq(f_proofread_unit_count + delta.proofread),
        f_updated_at.eq(now),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_chapter(id, "adjust chapter unit counters"));
    }

    accept!(())
}

/// Unpins all chapters for a comic except the one with the given excluded ID.
#[instrument(level = "info", skip_all)]
pub async fn unpin_others(
    conn: &mut RdbConn,
    comic_id: &str,
    excluded_id: &str,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    diesel::update(
        t_chapter
            .filter(f_comic_id.eq(comic_id))
            .filter(f_deleted_at.is_null())
            .filter(f_id.ne(excluded_id)),
    )
    .set((f_is_pinned.eq(false), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept!(())
}

// Converts a single `ChapterInfoRecord` into a `ChapterInfo`.
fn record_into_info(record: ChapterInfoRecord) -> BaseRest<ChapterInfo> {
    record.try_into()
}

// Converts a vector of `ChapterInfoRecord` values into `ChapterInfo`.
fn records_into_infos(
    records: Vec<ChapterInfoRecord>,
) -> BaseRest<Vec<ChapterInfo>> {
    records.into_iter().map(record_into_info).collect()
}

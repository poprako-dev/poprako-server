//! Typed database snapshots for Chapter creation transaction assertions.

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;

use poprako_rdb_core::RdbCore;

use crate::part_impl::repo::rdb_impl::schema::{
    t_assignment, t_chapter, t_chapter_workflow_record, t_comic, t_workset,
};

/// Persisted state affected by creating a Comic or Chapter in one Workset.
#[derive(Debug, PartialEq)]
pub struct CreationSnapshot {
    workset: (i32, i32, OffsetDateTime),

    comics: Vec<(String, i32, i32, i32, OffsetDateTime, OffsetDateTime)>,
    chapters: Vec<(String, String, i32, bool, String, OffsetDateTime)>,

    assignments: Vec<(String, String, String, OffsetDateTime)>,
    history: Vec<(
        String,
        String,
        Option<String>,
        String,
        serde_json::Value,
        OffsetDateTime,
    )>,
}

impl CreationSnapshot {
    /// Captures counters, allocation cursors, activity, pins, assignments and history.
    pub async fn load(shared: &RdbCore, workset_id: &str) -> Self {
        //
        let mut conn = shared.get().await.unwrap();

        let comic_ids = t_comic::table
            .filter(t_comic::f_workset_id.eq(workset_id))
            .select(t_comic::f_id);

        let chapter_ids = t_chapter::table
            .filter(t_chapter::f_comic_id.eq_any(comic_ids))
            .select(t_chapter::f_id);

        let workset = t_workset::table
            .find(workset_id)
            .select((
                t_workset::f_comic_count,
                t_workset::f_comic_next_index,
                t_workset::f_updated_at,
            ))
            .first(&mut conn)
            .await
            .unwrap();

        let comics = t_comic::table
            .filter(t_comic::f_workset_id.eq(workset_id))
            .order(t_comic::f_id)
            .select((
                t_comic::f_id,
                t_comic::f_index,
                t_comic::f_chapter_count,
                t_comic::f_chapter_next_index,
                t_comic::f_last_active_at,
                t_comic::f_updated_at,
            ))
            .load(&mut conn)
            .await
            .unwrap();

        let chapters = t_chapter::table
            .filter(t_chapter::f_comic_id.eq_any(comic_ids))
            .order(t_chapter::f_id)
            .select((
                t_chapter::f_id,
                t_chapter::f_comic_id,
                t_chapter::f_index,
                t_chapter::f_is_pinned,
                t_chapter::f_subtitle,
                t_chapter::f_updated_at,
            ))
            .load(&mut conn)
            .await
            .unwrap();

        let assignments = t_assignment::table
            .filter(t_assignment::f_chapter_id.eq_any(chapter_ids))
            .order(t_assignment::f_id)
            .select((
                t_assignment::f_id,
                t_assignment::f_chapter_id,
                t_assignment::f_user_id,
                t_assignment::f_updated_at,
            ))
            .load(&mut conn)
            .await
            .unwrap();

        let history = t_chapter_workflow_record::table
            .filter(t_chapter_workflow_record::f_chapter_id.eq_any(chapter_ids))
            .order(t_chapter_workflow_record::f_id)
            .select(t_chapter_workflow_record::all_columns)
            .load(&mut conn)
            .await
            .unwrap();

        Self {
            workset,
            comics,
            chapters,
            assignments,
            history,
        }
    }
}

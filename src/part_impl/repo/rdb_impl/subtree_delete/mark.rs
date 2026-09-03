//! Active hierarchy locking and atomic tombstone propagation.

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::subtree_delete::SubtreeDeleteScope;
use crate::part::repo::oper::subtree_delete::SubtreeRoot;
use crate::part_impl::repo::rdb_impl::schema::{
    t_chapter, t_comic, t_team, t_workset,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;

/// Build the expected error for a missing active root.
pub fn missing(
    error_key: &str,
    resource_kind: &str,
    resource_id: &str,
) -> BaseError {
    //
    let err_message = trl(error_key);

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        resource_kind,
        resource_id,
        "expected error: active subtree root not found",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}

/// Lock a comic and its surviving ancestry.
pub async fn lock_comic_scope(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<SubtreeDeleteScope> {
    //
    let (workset_id, team_id) = t_comic::table
        .inner_join(t_workset::table)
        .filter(t_comic::f_id.eq(id))
        .filter(t_comic::f_deleted_at.is_null())
        .select((t_comic::f_workset_id, t_workset::f_team_id))
        .get_result::<(String, String)>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| missing("error-comic-not-found", "comic", id))?;

    t_workset::table
        .filter(t_workset::f_id.eq(&workset_id))
        .select(t_workset::f_id)
        .for_update()
        .get_result::<String>(conn)
        .await
        .map_err(diesel)?;

    let comic_id = t_comic::table
        .filter(t_comic::f_id.eq(id))
        .filter(t_comic::f_deleted_at.is_null())
        .select(t_comic::f_id)
        .for_update()
        .get_result::<String>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| missing("error-comic-not-found", "comic", id))?;

    accept(SubtreeDeleteScope::Comic {
        comic_id,
        workset_id,
        team_id,
    })
}

/// Lock a chapter and its surviving ancestry.
pub async fn lock_chapter_scope(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<SubtreeDeleteScope> {
    //
    let (comic_id, workset_id, team_id) = t_chapter::table
        .inner_join(t_comic::table.inner_join(t_workset::table))
        .filter(t_chapter::f_id.eq(id))
        .filter(t_chapter::f_deleted_at.is_null())
        .select((
            t_chapter::f_comic_id,
            t_comic::f_workset_id,
            t_workset::f_team_id,
        ))
        .get_result::<(String, String, String)>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| missing("error-chapter-not-found", "chapter", id))?;

    t_workset::table
        .filter(t_workset::f_id.eq(&workset_id))
        .select(t_workset::f_id)
        .for_update()
        .get_result::<String>(conn)
        .await
        .map_err(diesel)?;

    t_comic::table
        .filter(t_comic::f_id.eq(&comic_id))
        .select(t_comic::f_id)
        .for_update()
        .get_result::<String>(conn)
        .await
        .map_err(diesel)?;

    let (chapter_id, was_pinned) = t_chapter::table
        .filter(t_chapter::f_id.eq(id))
        .filter(t_chapter::f_deleted_at.is_null())
        .select((t_chapter::f_id, t_chapter::f_is_pinned))
        .for_update()
        .get_result::<(String, bool)>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| missing("error-chapter-not-found", "chapter", id))?;

    accept(SubtreeDeleteScope::Chapter {
        chapter_id,
        comic_id,
        workset_id,
        team_id,
        was_pinned,
    })
}

/// Locks one active hierarchy root and returns its minimal ancestry.
pub async fn lock_scope(
    conn: &mut RdbConn,
    root: &SubtreeRoot<'_>,
) -> BaseRest<SubtreeDeleteScope> {
    //
    match root {
        //
        SubtreeRoot::Team { id } => {
            //
            let team_id = t_team::table
                .filter(t_team::f_id.eq(id))
                .filter(t_team::f_deleted_at.is_null())
                .select(t_team::f_id)
                .for_update()
                .get_result::<String>(conn)
                .await
                .optional()
                .map_err(diesel)?
                .ok_or_else(|| missing("error-team-not-found", "team", id))?;

            accept(SubtreeDeleteScope::Team { team_id })
        }

        SubtreeRoot::Workset { id } => {
            //
            let (workset_id, team_id) = t_workset::table
                .filter(t_workset::f_id.eq(id))
                .filter(t_workset::f_deleted_at.is_null())
                .select((t_workset::f_id, t_workset::f_team_id))
                .for_update()
                .get_result::<(String, String)>(conn)
                .await
                .optional()
                .map_err(diesel)?
                .ok_or_else(|| {
                    missing("error-workset-not-found", "workset", id)
                })?;

            accept(SubtreeDeleteScope::Workset {
                workset_id,
                team_id,
            })
        }

        SubtreeRoot::Comic { id } => lock_comic_scope(conn, id).await,

        SubtreeRoot::Chapter { id } => lock_chapter_scope(conn, id).await,
    }
}

/// Marks one locked aggregate and all hierarchy descendants with one timestamp.
#[expect(
    clippy::too_many_lines,
    reason = "each hierarchy level requires ordered tombstone updates"
)]
pub async fn mark_scope(
    conn: &mut RdbConn,
    scope: &SubtreeDeleteScope,
) -> BaseRest<()> {
    //
    let deleted_at = OffsetDateTime::now_utc();

    match scope {
        //
        SubtreeDeleteScope::Team { team_id } => {
            //
            diesel::update(
                t_team::table
                    .filter(t_team::f_id.eq(team_id))
                    .filter(t_team::f_deleted_at.is_null()),
            )
            .set(t_team::f_deleted_at.eq(deleted_at))
            .execute(conn)
            .await
            .map_err(diesel)?;

            diesel::update(
                t_workset::table
                    .filter(t_workset::f_team_id.eq(team_id))
                    .filter(t_workset::f_deleted_at.is_null()),
            )
            .set(t_workset::f_deleted_at.eq(deleted_at))
            .execute(conn)
            .await
            .map_err(diesel)?;

            let workset_ids = t_workset::table
                .filter(t_workset::f_team_id.eq(team_id))
                .select(t_workset::f_id);

            diesel::update(
                t_comic::table
                    .filter(t_comic::f_workset_id.eq_any(workset_ids))
                    .filter(t_comic::f_deleted_at.is_null()),
            )
            .set(t_comic::f_deleted_at.eq(deleted_at))
            .execute(conn)
            .await
            .map_err(diesel)?;

            let comic_ids = t_comic::table
                .inner_join(t_workset::table)
                .filter(t_workset::f_team_id.eq(team_id))
                .select(t_comic::f_id);

            diesel::update(
                t_chapter::table
                    .filter(t_chapter::f_comic_id.eq_any(comic_ids))
                    .filter(t_chapter::f_deleted_at.is_null()),
            )
            .set(t_chapter::f_deleted_at.eq(deleted_at))
            .execute(conn)
            .await
            .map_err(diesel)?;
        }

        SubtreeDeleteScope::Workset { workset_id, .. } => {
            //
            diesel::update(
                t_workset::table
                    .filter(t_workset::f_id.eq(workset_id))
                    .filter(t_workset::f_deleted_at.is_null()),
            )
            .set(t_workset::f_deleted_at.eq(deleted_at))
            .execute(conn)
            .await
            .map_err(diesel)?;

            diesel::update(
                t_comic::table
                    .filter(t_comic::f_workset_id.eq(workset_id))
                    .filter(t_comic::f_deleted_at.is_null()),
            )
            .set(t_comic::f_deleted_at.eq(deleted_at))
            .execute(conn)
            .await
            .map_err(diesel)?;

            let comic_ids = t_comic::table
                .filter(t_comic::f_workset_id.eq(workset_id))
                .select(t_comic::f_id);

            diesel::update(
                t_chapter::table
                    .filter(t_chapter::f_comic_id.eq_any(comic_ids))
                    .filter(t_chapter::f_deleted_at.is_null()),
            )
            .set(t_chapter::f_deleted_at.eq(deleted_at))
            .execute(conn)
            .await
            .map_err(diesel)?;
        }

        SubtreeDeleteScope::Comic { comic_id, .. } => {
            //
            diesel::update(
                t_comic::table
                    .filter(t_comic::f_id.eq(comic_id))
                    .filter(t_comic::f_deleted_at.is_null()),
            )
            .set(t_comic::f_deleted_at.eq(deleted_at))
            .execute(conn)
            .await
            .map_err(diesel)?;

            diesel::update(
                t_chapter::table
                    .filter(t_chapter::f_comic_id.eq(comic_id))
                    .filter(t_chapter::f_deleted_at.is_null()),
            )
            .set(t_chapter::f_deleted_at.eq(deleted_at))
            .execute(conn)
            .await
            .map_err(diesel)?;
        }

        SubtreeDeleteScope::Chapter { .. } => {
            //
            return Err(BaseError::Unrecoverable {
                message: "direct chapter deletion must not create a tombstone"
                    .into(),
            });
        }
    }

    accept(())
}

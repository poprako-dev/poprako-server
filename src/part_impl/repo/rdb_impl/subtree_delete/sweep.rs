//! Tombstone claiming and dependency-ordered physical deletion.

use diesel::dsl::{exists, not};
use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
};
use diesel_async::RunQueryDsl as _;

use poprako_rdb_core::RdbConn;

use crate::model::read::proj::subtree_delete::{
    SubtreeDeleteScope, SubtreeDeleteSweepTarget,
};
use crate::part_impl::repo::rdb_impl::schema::{
    t_announcement, t_assignment, t_assignment_invitation, t_chapter,
    t_chapter_workflow_record, t_comic, t_comic_archive, t_comment, t_member,
    t_member_invitation, t_page, t_team, t_term, t_termbase, t_unit, t_workset,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;

/// Delete relational rows owned by one chapter.
pub async fn delete_chapter_rows(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<()> {
    //
    let page_ids = t_page::table
        .filter(t_page::f_chapter_id.eq(chapter_id))
        .select(t_page::f_id);

    diesel::delete(
        t_assignment_invitation::table
            .filter(t_assignment_invitation::f_chapter_id.eq(chapter_id)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_assignment::table.filter(t_assignment::f_chapter_id.eq(chapter_id)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_chapter_workflow_record::table
            .filter(t_chapter_workflow_record::f_chapter_id.eq(chapter_id)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(t_unit::table.filter(t_unit::f_page_id.eq_any(page_ids)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(t_page::table.filter(t_page::f_chapter_id.eq(chapter_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(t_chapter::table.filter(t_chapter::f_id.eq(chapter_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Delete relational rows owned by one comic.
pub async fn delete_comic_rows(
    conn: &mut RdbConn,
    comic_id: &str,
) -> BaseRest<()> {
    //
    let termbase_ids = t_termbase::table
        .filter(t_termbase::f_comic_id.eq(comic_id))
        .select(t_termbase::f_id);

    diesel::delete(
        t_term::table.filter(t_term::f_termbase_id.eq_any(termbase_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_termbase::table.filter(t_termbase::f_comic_id.eq(comic_id)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_comic_archive::table
            .filter(t_comic_archive::f_source_comic_id.eq(comic_id)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(t_comic::table.filter(t_comic::f_id.eq(comic_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Delete relational rows owned by one team.
pub async fn delete_team_rows(
    conn: &mut RdbConn,
    team_id: &str,
) -> BaseRest<()> {
    //
    let termbase_ids = t_termbase::table
        .filter(t_termbase::f_team_id.eq(team_id))
        .select(t_termbase::f_id);

    diesel::delete(
        t_term::table.filter(t_term::f_termbase_id.eq_any(termbase_ids)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(t_termbase::table.filter(t_termbase::f_team_id.eq(team_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(
        t_comic_archive::table.filter(t_comic_archive::f_team_id.eq(team_id)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(
        t_announcement::table.filter(t_announcement::f_team_id.eq(team_id)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(t_comment::table.filter(t_comment::f_team_id.eq(team_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(
        t_member_invitation::table
            .filter(t_member_invitation::f_team_id.eq(team_id)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::delete(t_member::table.filter(t_member::f_team_id.eq(team_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(t_team::table.filter(t_team::f_id.eq(team_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Claims the next eligible tombstone in dependency order.
pub async fn claim(
    conn: &mut RdbConn,
) -> BaseRest<Option<SubtreeDeleteSweepTarget>> {
    //
    let chapter_id = t_chapter::table
        .filter(t_chapter::f_deleted_at.is_not_null())
        .select(t_chapter::f_id)
        .order((t_chapter::f_deleted_at.asc(), t_chapter::f_id.asc()))
        .for_update()
        .skip_locked()
        .first::<String>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    if let Some(id) = chapter_id {
        return accept(Some(SubtreeDeleteSweepTarget::Chapter { id }));
    }

    let comic_id = t_comic::table
        .filter(t_comic::f_deleted_at.is_not_null())
        .filter(not(exists(
            t_chapter::table.filter(t_chapter::f_comic_id.eq(t_comic::f_id)),
        )))
        .select(t_comic::f_id)
        .order((t_comic::f_deleted_at.asc(), t_comic::f_id.asc()))
        .for_update()
        .skip_locked()
        .first::<String>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    if let Some(id) = comic_id {
        return accept(Some(SubtreeDeleteSweepTarget::Comic { id }));
    }

    let workset_id = t_workset::table
        .filter(t_workset::f_deleted_at.is_not_null())
        .filter(not(exists(
            t_comic::table.filter(t_comic::f_workset_id.eq(t_workset::f_id)),
        )))
        .select(t_workset::f_id)
        .order((t_workset::f_deleted_at.asc(), t_workset::f_id.asc()))
        .for_update()
        .skip_locked()
        .first::<String>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    if let Some(id) = workset_id {
        return accept(Some(SubtreeDeleteSweepTarget::Workset { id }));
    }

    let team_id = t_team::table
        .filter(t_team::f_deleted_at.is_not_null())
        .filter(not(exists(
            t_workset::table.filter(t_workset::f_team_id.eq(t_team::f_id)),
        )))
        .select(t_team::f_id)
        .order((t_team::f_deleted_at.asc(), t_team::f_id.asc()))
        .for_update()
        .skip_locked()
        .first::<String>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    accept(team_id.map(|id| SubtreeDeleteSweepTarget::Team { id }))
}

/// Lists only the object identifiers needed before deleting one chapter.
pub async fn list_page_ids(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<Vec<String>> {
    //
    let ids = t_page::table
        .filter(t_page::f_chapter_id.eq(chapter_id))
        .select(t_page::f_id)
        .order(t_page::f_id.asc())
        .load::<String>(conn)
        .await
        .map_err(diesel)?;

    accept(ids)
}

/// Physically deletes the direct dependants and row for one claimed target.
pub async fn delete_target(
    conn: &mut RdbConn,
    target: &SubtreeDeleteSweepTarget,
) -> BaseRest<()> {
    //
    match target {
        //
        SubtreeDeleteSweepTarget::Chapter { id } => {
            delete_chapter_rows(conn, id).await?;
        }

        SubtreeDeleteSweepTarget::Comic { id } => {
            delete_comic_rows(conn, id).await?;
        }

        SubtreeDeleteSweepTarget::Workset { id } => {
            //
            diesel::delete(t_workset::table.filter(t_workset::f_id.eq(id)))
                .execute(conn)
                .await
                .map_err(diesel)?;
        }

        SubtreeDeleteSweepTarget::Team { id } => {
            delete_team_rows(conn, id).await?;
        }
    }

    accept(())
}

/// Hard-deletes a directly requested active Chapter subtree.
pub async fn delete_active_scope(
    conn: &mut RdbConn,
    scope: &SubtreeDeleteScope,
) -> BaseRest<()> {
    //
    let SubtreeDeleteScope::Chapter { chapter_id, .. } = scope else {
        //
        return Err(BaseError::Unrecoverable {
            message: "only a direct chapter may bypass subtree tombstones"
                .into(),
        });
    };

    delete_chapter_rows(conn, chapter_id).await
}

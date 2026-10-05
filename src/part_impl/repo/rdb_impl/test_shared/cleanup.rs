use diesel::prelude::{QueryDsl as _, TextExpressionMethods as _};
use diesel_async::RunQueryDsl as _;

use poprako_rdb_core::RdbCore;

use crate::result::{BaseRest, accept};
use crate::shared::result::diesel as diesel_error;

// Keep each cleanup statement typed while sharing execution and error handling.
macro_rules! delete_fixture_rows {
    ($conn:expr, $pattern:expr, $table:ident, $column:ident) => {
        diesel::delete(
            crate::part_impl::repo::rdb_impl::schema::$table::table.filter(
                crate::part_impl::repo::rdb_impl::schema::$table::$column
                    .like($pattern),
            ),
        )
        .execute($conn)
        .await
        .map_err(diesel_error)?
    };
}

// Assert concrete tables using their generated Diesel schema expressions.
macro_rules! assert_fixture_table_empty {
    ($conn:expr, $pattern:expr, $table:ident, $column:ident) => {{
        let count = crate::part_impl::repo::rdb_impl::schema::$table::table
            .filter(
                crate::part_impl::repo::rdb_impl::schema::$table::$column
                    .like($pattern),
            )
            .count()
            .get_result::<i64>($conn)
            .await
            .map_err(diesel_error)?;

        assert_eq!(count, 0, stringify!($table));
    }};
}

/// Removes fixture rows in dependency order for one test prefix.
///
/// # Errors
/// Returns database connection or typed query errors.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub async fn cleanup(shared: &RdbCore, prefix: &str) -> BaseRest<()> {
    //
    let mut conn = shared.get().await?;

    let id_pattern = format!("{}%", prefix);

    delete_fixture_rows!(&mut conn, &id_pattern, t_comment, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_announcement, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_assignment_invitation, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_assignment, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_unit, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_page, f_id);

    delete_fixture_rows!(
        &mut conn,
        &id_pattern,
        t_chapter_workflow_record,
        f_chapter_id
    );

    delete_fixture_rows!(&mut conn, &id_pattern, t_chapter, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_term, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_termbase, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_comic, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_workset, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_member_invitation, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_member, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_system_mail, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_local_message, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_team, f_id);

    delete_fixture_rows!(&mut conn, &id_pattern, t_user, f_id);

    accept(())
}

/// Verifies cleanup removed every fixture row covered by the prefix.
///
/// # Panics
/// Panics if matching fixture records remain after cleanup.
///
/// # Errors
/// Returns database connection or typed query errors.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub async fn assert_no_leftovers(
    shared: &RdbCore,
    prefix: &str,
) -> BaseRest<()> {
    //
    let mut conn = shared.get().await?;

    let id_pattern = format!("{}%", prefix);

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_announcement, f_id);

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_assignment, f_id);

    assert_fixture_table_empty!(
        &mut conn,
        &id_pattern,
        t_assignment_invitation,
        f_id
    );

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_chapter, f_id);

    assert_fixture_table_empty!(
        &mut conn,
        &id_pattern,
        t_chapter_workflow_record,
        f_chapter_id
    );

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_comic, f_id);

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_comment, f_id);

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_local_message, f_id);

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_member, f_id);

    assert_fixture_table_empty!(
        &mut conn,
        &id_pattern,
        t_member_invitation,
        f_id
    );

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_page, f_id);

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_system_mail, f_id);

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_team, f_id);

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_unit, f_id);

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_user, f_id);

    assert_fixture_table_empty!(&mut conn, &id_pattern, t_workset, f_id);

    accept(())
}

use diesel::prelude::{QueryDsl as _, TextExpressionMethods as _};
use diesel_async::RunQueryDsl as _;

use poprako_rdb_core::RdbCore;

use crate::part_impl::repo::rdb_impl::schema;
use crate::result::{BaseRest, accept};
use crate::shared::result::diesel as diesel_error;

pub async fn cleanup(shared: &RdbCore, prefix: &str) -> BaseRest<()> {
    //
    let mut conn = shared.get().await?;

    let id_pattern = format!("{}%", prefix);

    diesel::delete(
        schema::t_comment::table
            .filter(schema::t_comment::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_announcement::table
            .filter(schema::t_announcement::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_assignment_invitation::table
            .filter(schema::t_assignment_invitation::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_assignment::table
            .filter(schema::t_assignment::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_unit::table.filter(schema::t_unit::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_page::table.filter(schema::t_page::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(schema::t_chapter_workflow_record::table.filter(
        schema::t_chapter_workflow_record::f_chapter_id.like(&id_pattern),
    ))
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_chapter::table
            .filter(schema::t_chapter::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_term::table.filter(schema::t_term::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_termbase::table
            .filter(schema::t_termbase::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_comic::table.filter(schema::t_comic::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_workset::table
            .filter(schema::t_workset::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_member_invitation::table
            .filter(schema::t_member_invitation::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_member::table
            .filter(schema::t_member::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_system_mail::table
            .filter(schema::t_system_mail::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_local_message::table
            .filter(schema::t_local_message::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_team::table.filter(schema::t_team::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_user::table.filter(schema::t_user::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    accept(())
}

pub async fn assert_no_leftovers(
    shared: &RdbCore,
    prefix: &str,
) -> BaseRest<()> {
    //
    let mut conn = shared.get().await?;

    let id_pattern = format!("{}%", prefix);

    let announcement_count = schema::t_announcement::table
        .filter(schema::t_announcement::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let assignment_count = schema::t_assignment::table
        .filter(schema::t_assignment::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let assignment_invitation_count = schema::t_assignment_invitation::table
        .filter(schema::t_assignment_invitation::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let chapter_count = schema::t_chapter::table
        .filter(schema::t_chapter::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let chapter_workflow_record_count =
        schema::t_chapter_workflow_record::table
            .filter(
                schema::t_chapter_workflow_record::f_chapter_id
                    .like(&id_pattern),
            )
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .map_err(diesel_error)?;

    let comic_count = schema::t_comic::table
        .filter(schema::t_comic::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let comment_count = schema::t_comment::table
        .filter(schema::t_comment::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let local_message_count = schema::t_local_message::table
        .filter(schema::t_local_message::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let member_count = schema::t_member::table
        .filter(schema::t_member::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let member_invitation_count = schema::t_member_invitation::table
        .filter(schema::t_member_invitation::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let page_count = schema::t_page::table
        .filter(schema::t_page::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let system_mail_count = schema::t_system_mail::table
        .filter(schema::t_system_mail::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let team_count = schema::t_team::table
        .filter(schema::t_team::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let unit_count = schema::t_unit::table
        .filter(schema::t_unit::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let user_count = schema::t_user::table
        .filter(schema::t_user::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    let workset_count = schema::t_workset::table
        .filter(schema::t_workset::f_id.like(&id_pattern))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .map_err(diesel_error)?;

    assert_eq!(announcement_count, 0);

    assert_eq!(assignment_count, 0);

    assert_eq!(assignment_invitation_count, 0);

    assert_eq!(chapter_count, 0);

    assert_eq!(chapter_workflow_record_count, 0);

    assert_eq!(comic_count, 0);

    assert_eq!(comment_count, 0);

    assert_eq!(local_message_count, 0);

    assert_eq!(member_count, 0);

    assert_eq!(member_invitation_count, 0);

    assert_eq!(page_count, 0);

    assert_eq!(system_mail_count, 0);

    assert_eq!(team_count, 0);

    assert_eq!(unit_count, 0);

    assert_eq!(user_count, 0);

    assert_eq!(workset_count, 0);

    accept(())
}

use async_trait::async_trait;

use crate::model::comment_model;
use crate::model::user_model;
use crate::part_impl::repo::rdb_impl::incl::{self, Incl, UserByIds};
use crate::part_impl::shared::RdbConn;
use crate::result::RegularResult;
use crate::value::comment::CommentInclOpt;

/// Include struct for eager-loading [`UserInfo`] data into [`CommentInfo`] query results.
struct CommentUserIncl;

#[async_trait]
impl Incl for CommentUserIncl {
    type Owner = comment_model::Info;
    type Related = user_model::Info;
    type Query = UserByIds;

    fn resolve_key(owner: &comment_model::Info) -> Option<&str> {
        Some(&owner.user_id)
    }

    fn inject(
        owner: &mut comment_model::Info,
        related: Option<user_model::Info>,
    ) {
        owner.user = related;
    }
}

/// Populates comment query results with eagerly-loaded user data.
pub async fn populate_comment_incls(
    conn: &mut RdbConn,
    infos: &mut [comment_model::Info],
    incl_opt: &[CommentInclOpt],
) -> RegularResult<()> {
    //
    if incl_opt.contains(&CommentInclOpt::User) {
        incl::populate::<CommentUserIncl>(conn, infos).await?;
    }

    Ok(())
}

use crate::model::comment::CommentInfo;
use crate::model::user::UserInfo;
use crate::part_impl::repo::rdb_impl::incl::{self, Incl, UserByIds};
use crate::part_impl::shared::RdbConn;
use crate::result::RegularResult;
use crate::value::comment::CommentInclOpt;

/// Include struct for eager-loading [`UserInfo`] data into [`CommentInfo`] query results.
struct CommentUserIncl;

impl Incl for CommentUserIncl {
    type Owner = CommentInfo;
    type Related = UserInfo;
    type Query = UserByIds;

    fn resolve_key(owner: &CommentInfo) -> Option<&str> {
        Some(&owner.user_id)
    }

    fn inject(owner: &mut CommentInfo, related: Option<UserInfo>) {
        owner.user = related;
    }
}

/// Populates comment query results with eagerly-loaded user data.
pub async fn populate_comment_incls(
    conn: &mut RdbConn,
    infos: &mut [CommentInfo],
    incl_opt: &[CommentInclOpt],
) -> RegularResult<()> {
    //
    if incl_opt.contains(&CommentInclOpt::User) {
        incl::populate::<CommentUserIncl>(conn, infos).await?;
    }

    Ok(())
}

use crate::model::read::proj::comment::CommentInfo;
use crate::part_impl::repo::rdb_impl::incl::framework::UserByIds;
use crate::value::comment::CommentInclOpt;

preloadable! {
    owner: CommentInfo,
    opt: CommentInclOpt,
    populate: populate_comment_incls,
    variants: {
        User => UserByIds {
            resolve: path [] => user_id,
            inject: path [] => user,
        },
    },
}

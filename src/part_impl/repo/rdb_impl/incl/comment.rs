use crate::model::read::proj::comment::CommentInfo;
use crate::part_impl::repo::rdb_impl::incl::framework::{
    BatchByIds, Incl, UserByIds, populate,
};
use crate::result::BaseRest;
use crate::shared::RdbConn;
use crate::value::comment::CommentInclOpt;
use crate::value::incl::expand_incl_opts;

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

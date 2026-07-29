use crate::model::read::proj::announcement::AnnouncementInfo;
use crate::part_impl::repo::rdb_impl::incl::framework::{
    BatchByIds, Incl, UserByIds, populate,
};
use crate::part_impl::shared::RdbConn;
use crate::result::BaseRest;
use crate::value::announcement::AnnouncementInclOpt;
use crate::value::incl::expand_incl_opts;

preloadable! {
    owner: AnnouncementInfo,
    opt: AnnouncementInclOpt,
    populate: populate_announcement_incls,
    variants: {
        User => UserByIds {
            resolve: path [] => user_id,
            inject: path [] => user,
        },
    },
}

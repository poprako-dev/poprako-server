use crate::model::read::proj::announcement::AnnouncementInfo;
use crate::part_impl::repo::rdb_impl::incl::framework::UserByIds;
use crate::value::announcement::AnnouncementInclOpt;

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

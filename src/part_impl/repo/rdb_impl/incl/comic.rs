use crate::model::read::proj::comic::ComicInfo;
use crate::part_impl::repo::rdb_impl::incl::framework::{
    TeamByIds, UserByIds, WorksetByIds,
};
use crate::value::comic::ComicInclOpt;

preloadable! {
    owner: ComicInfo,
    opt: ComicInclOpt,
    populate: populate_comic_incls,
    variants: {
        Workset => WorksetByIds {
            resolve: path [] => workset_id,
            inject: path [] => workset,
        },
        WorksetTeam => TeamByIds {
            resolve: path [workset] => team_id,
            inject: path [] => team,
        },
        Creator => UserByIds {
            resolve: path [] => creator_id,
            inject: path [] => creator,
        },
    },
}

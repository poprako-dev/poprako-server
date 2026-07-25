use crate::model::comic::ComicInfo;
use crate::part_impl::repo::rdb_impl::incl::framework::{BatchByIds, Incl, TeamByIds, UserByIds, WorksetByIds, populate};
use crate::part_impl::shared::RdbConn;
use crate::result::BaseResult;
use crate::value::comic::ComicInclOpt;
use crate::value::incl::expand_incl_opts;

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

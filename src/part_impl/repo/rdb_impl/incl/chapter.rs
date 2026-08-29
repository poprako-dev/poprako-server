use poprako_rdb_core::RdbConn;

use crate::model::read::proj::chapter::ChapterInfo;
use crate::part_impl::repo::rdb_impl::incl::framework::{
    BatchByIds, ComicByIds, Incl, TeamByIds, UserByIds, WorksetByIds, populate,
};
use crate::result::BaseRest;
use crate::value::chapter::ChapterInclOpt;
use crate::value::incl::expand_incl_opts;

preloadable! {
    owner: ChapterInfo,
    opt: ChapterInclOpt,
    populate: populate_chapter_incls,
    variants: {
        Comic => ComicByIds {
            resolve: path [] => comic_id,
            inject: path [] => comic,
        },
        ComicWorkset => WorksetByIds {
            resolve: path [comic] => workset_id,
            inject: path [comic] => workset,
        },
        ComicWorksetTeam => TeamByIds {
            resolve: path [comic, workset] => team_id,
            inject: path [comic] => team,
        },
        ComicCreator => UserByIds {
            resolve: path [comic] => creator_id,
            inject: path [comic] => creator,
        },
        Creator => UserByIds {
            resolve: path [] => creator_id,
            inject: path [] => creator,
        },
    },
}

use crate::model::assignment::AssignmentInfo;
use crate::part_impl::repo::rdb_impl::incl::framework::{
    BatchByIds, ChapterByIds, ComicByIds, Incl, TeamByIds, UserByIds,
    WorksetByIds, populate,
};
use crate::part_impl::shared::RdbConn;
use crate::result::BaseResult;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::incl::expand_incl_opts;

preloadable! {
    owner: AssignmentInfo,
    opt: AssignmentInclOpt,
    populate: populate_assignment_incls,
    variants: {
        User => UserByIds {
            resolve: path [] => user_id,
            inject: path [] => user,
        },
        Chapter => ChapterByIds {
            resolve: path [] => chapter_id,
            inject: path [] => chapter,
        },
        ChapterComic => ComicByIds {
            resolve: path [chapter] => comic_id,
            inject: path [chapter] => comic,
        },
        ChapterComicWorkset => WorksetByIds {
            resolve: path [chapter, comic] => workset_id,
            inject: path [chapter, comic] => workset,
        },
        ChapterComicWorksetTeam => TeamByIds {
            resolve: path [chapter, comic, workset] => team_id,
            inject: path [chapter, comic] => team,
        },
        ChapterCreator => UserByIds {
            resolve: path [chapter] => creator_id,
            inject: path [chapter] => creator,
        },
        ChapterComicCreator => UserByIds {
            resolve: path [chapter, comic] => creator_id,
            inject: path [chapter, comic] => creator,
        },
    },
}

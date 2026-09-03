use poprako_orchestra::drive;

use crate::part::repo::oper::comic::{
    AllocComicChapterIndex, CreateComic, GetComicInfo, GetComicInfoExcluded,
    ListComicInfos, TouchComicLastActive, UpdateComic, UpdateComicChapterCount,
};
use crate::result::BaseError;

/// Comic repository operations.
///
/// Independent reads and externally confirmed updates use [`poprako_orchestra::Run`].
/// Coordinated reads, mutations, and pessimistic locks use [`poprako_orchestra::Step`].
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a, 'b> GetComicInfo<'a, 'b>,
        for<'a> ListComicInfos<'a>,
        for<'a> UpdateComic<'a>,
    ),
    step(
        for<'a, 'b> GetComicInfo<'a, 'b>,
        for<'a> ListComicInfos<'a>,
        for<'a, 'b> GetComicInfoExcluded<'a, 'b>,
        for<'a> CreateComic<'a>,
        for<'a> AllocComicChapterIndex<'a>,
        for<'a> UpdateComicChapterCount<'a>,
        for<'a> TouchComicLastActive<'a>,
    ),
)]
pub trait ComicRepo<C> {}

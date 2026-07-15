use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::comic::{
    AllocComicChapterIndex, CreateComic, DeleteComic, GetComicInfo,
    GetComicInfoExcluded, ListComicInfos, ListComicInfosExcluded,
    MarkComicCoverUploaded, ReserveComicCover, TouchComicLastActive,
    UpdateComic, UpdateComicChapterCount,
};
use crate::result::BaseError;

/// Comic repository operations.
///
/// Independent reads and externally confirmed updates use [`Run`].
/// Coordinated reads, mutations, and pessimistic locks use [`Step`].
pub trait ComicRepo<C>:
    for<'a, 'b> Run<GetComicInfo<'a, 'b>, Error = BaseError>
    + for<'a> Run<ListComicInfos<'a>, Error = BaseError>
    + for<'a> Run<UpdateComic<'a>, Error = BaseError>
    + for<'a> Run<MarkComicCoverUploaded<'a>, Error = BaseError>
    + for<'a, 'b> Step<GetComicInfo<'a, 'b>, C, Error = BaseError>
    + for<'a> Step<ListComicInfos<'a>, C, Error = BaseError>
    + for<'a, 'b> Step<GetComicInfoExcluded<'a, 'b>, C, Error = BaseError>
    + for<'a> Step<ListComicInfosExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<CreateComic<'a>, C, Error = BaseError>
    + for<'a> Step<ReserveComicCover<'a>, C, Error = BaseError>
    + for<'a> Step<MarkComicCoverUploaded<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteComic<'a>, C, Error = BaseError>
    + for<'a> Step<AllocComicChapterIndex<'a>, C, Error = BaseError>
    + for<'a> Step<UpdateComicChapterCount<'a>, C, Error = BaseError>
    + for<'a> Step<TouchComicLastActive<'a>, C, Error = BaseError>
{
}

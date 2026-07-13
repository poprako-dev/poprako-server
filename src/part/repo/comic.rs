use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::comic::{
    AllocateComicChapterIndex, CreateComic, DeleteComic, GetComicInfo,
    GetComicInfoExcluded, ListComicInfos, ListComicInfosExcluded,
    MarkComicCoverUploaded, ReserveComicCover, TouchComicLastActive,
    UpdateComic, UpdateComicChapterCount,
};
use crate::result::RegularError;

/// Comic repository operations.
///
/// Independent reads and externally confirmed updates use [`Run`].
/// Coordinated reads, mutations, and pessimistic locks use [`Step`].
pub trait ComicRepo<C>:
    for<'a, 'b> Run<GetComicInfo<'a, 'b>, Error = RegularError>
    + for<'a> Run<ListComicInfos<'a>, Error = RegularError>
    + for<'a> Run<UpdateComic<'a>, Error = RegularError>
    + for<'a> Run<MarkComicCoverUploaded<'a>, Error = RegularError>
    + for<'a, 'b> Step<GetComicInfo<'a, 'b>, C, Error = RegularError>
    + for<'a> Step<ListComicInfos<'a>, C, Error = RegularError>
    + for<'a, 'b> Step<GetComicInfoExcluded<'a, 'b>, C, Error = RegularError>
    + for<'a> Step<ListComicInfosExcluded<'a>, C, Error = RegularError>
    + for<'a> Step<CreateComic<'a>, C, Error = RegularError>
    + for<'a> Step<ReserveComicCover<'a>, C, Error = RegularError>
    + for<'a> Step<MarkComicCoverUploaded<'a>, C, Error = RegularError>
    + for<'a> Step<DeleteComic<'a>, C, Error = RegularError>
    + for<'a> Step<AllocateComicChapterIndex<'a>, C, Error = RegularError>
    + for<'a> Step<UpdateComicChapterCount<'a>, C, Error = RegularError>
    + for<'a> Step<TouchComicLastActive<'a>, C, Error = RegularError>
{
}

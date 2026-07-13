use poprako_orchestra::Oper;

use crate::model::comic_archive::{ComicArchiveSnapshot, ComicArchiveWrite};

pub struct GetComicArchiveSnapshotExcluded<'a> {
    pub comic_id: &'a str,
}

impl<'a> Oper for GetComicArchiveSnapshotExcluded<'a> {
    type Output = ComicArchiveSnapshot;
}

pub struct CommitComicArchive<'a> {
    pub write: &'a ComicArchiveWrite,
}

impl<'a> Oper for CommitComicArchive<'a> {
    type Output = ();
}

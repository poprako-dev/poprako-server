use poprako_orchestra::Oper;

use crate::model::comic_archive::{ComicArchiveSnapshot, ComicArchiveWrite};

pub struct GetComicArchiveSnapshotExcluded<'a> {
    pub comic_id: &'a str,
}

impl Oper for GetComicArchiveSnapshotExcluded<'_> {
    type Output = ComicArchiveSnapshot;
}

pub struct CommitComicArchive<'a> {
    pub write: &'a ComicArchiveWrite,
}

impl Oper for CommitComicArchive<'_> {
    type Output = ();
}

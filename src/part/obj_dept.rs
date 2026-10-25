//! Business-object markers registered with ObjDept.

/// source page-image object association.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageSourceImage;

/// User-avatar object association.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserAvatar;

/// Team-avatar object association.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TeamAvatar;

/// Comic-cover object association.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComicCover;

/// Chapter artwork file association.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChapterArtwork;

/// Composite artwork page image association.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageArtworkImage;

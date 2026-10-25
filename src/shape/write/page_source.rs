//! Borrowed page write payloads.
//! Owners remain alive through persistence; existing data is not copied.

/// Complete source filenames for the selected pages.
pub struct PageSourceRawIdentsReplShape<'a> {
    /// Paired page IDs and filenames; None means no original identifier.
    pub idents: Vec<(&'a str, Option<&'a str>)>,
}

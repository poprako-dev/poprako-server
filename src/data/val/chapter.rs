//! Val DTOs for the chapter domain.
//! Data transfer objects for chapter use cases.
/// Return value from a successful chapter creation.
pub struct CreateChapterVal {
    /// Unique identifier of the newly created chapter.
    pub id: String,
}

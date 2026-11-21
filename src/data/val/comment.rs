//! Val DTOs for the comment domain.
//! Data transfer objects for comment use cases.
/// Return value from creating a comment.
pub struct CreateCommentVal {
    /// Identifier of the newly created comment.
    pub id: String,
}

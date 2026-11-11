//! Val DTOs for the member domain.
//! Data transfer objects for member use cases.
/// Return value from creating a member.
pub struct CreateMemberVal {
    /// Identifier of the created member.
    pub id: String,
}

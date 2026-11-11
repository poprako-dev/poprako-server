//! Val DTOs for the announcement domain.
//! Data transfer objects for announcement use cases.
/// Return value from creating an announcement.
pub struct CreateAnnouncementVal {
    /// Identifier of the created announcement.
    pub id: String,
}

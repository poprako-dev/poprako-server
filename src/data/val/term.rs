//! Val DTOs for the term domain.
//! Request and response DTOs for terminology-entry use cases.
/// Return value from creating a terminology entry.
#[cfg_attr(test, derive(Debug))]
pub struct CreateTermVal {
    /// Identifier of the newly created terminology entry.
    pub id: String,
}

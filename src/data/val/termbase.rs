//! Val DTOs for the termbase domain.
//! Request and response DTOs for terminology-base use cases.
/// Return value from creating a terminology base.
#[cfg_attr(test, derive(Debug))]
pub struct CreateTermbaseVal {
    /// Identifier of the newly created terminology base.
    pub id: String,
}

//! Stable identities assigned by a committed Unit save.

/// One batch-local identity and its permanent Unit identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitSaveCreatedUnitId {
    /// Identity supplied in the create operation.
    pub local_id: String,
    /// Permanent identity assigned by the server.
    pub unit_id: String,
}

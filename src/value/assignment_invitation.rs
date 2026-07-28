//! Value types for assignment invitation aggregates.

/// Consumption-status filtering mode for listing assignment invitations.
pub enum AssignmentInvitationStatus {
    //
    /// Include invitations regardless of consumption status.
    All,

    /// Include only invitations that have not yet been consumed.
    Pending,

    /// Include only invitations that have already been consumed.
    Used,
}

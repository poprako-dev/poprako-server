/// Content-field permissions derived from the current assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitEditPerm {
    //
    /// Whether translation content may be changed.
    pub can_translate: bool,
    /// Whether revision content may be changed.
    pub can_proofread: bool,
}

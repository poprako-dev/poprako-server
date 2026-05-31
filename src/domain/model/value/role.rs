/// Role represents **ONE** specific role that a member or
/// assignment can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RoleFlag {
    RawProvider = 1 << 0,
    Translator = 1 << 1,
    Proofreader = 1 << 2,
    Typesetter = 1 << 3,
    Redrawer = 1 << 4,
    Reviewer = 1 << 5,
    Publisher = 1 << 6,
    Admin = 1 << 7,
    Assistant = 1 << 8,
}

impl From<RoleFlag> for u32 {
    fn from(val: RoleFlag) -> Self {
        val as u32
    }
}

/// RoleMask represents a combination of multiple roles that a member or
/// assignment can have. It is implemented as a bitmask, where each bit represents
/// a specific role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleMask(u32);

impl RoleMask {
    /// Bitmask of all valid role bits (bit 0 through 8).
    const VALID_BITS: u32 = (1 << 9) - 1;

    pub fn has_any_role(&self, flags: &[RoleFlag]) -> bool {
        for f in flags {
            if self.has_role(*f) {
                return true;
            }
        }
        false
    }

    pub fn has_every_role(&self, flags: &[RoleFlag]) -> bool {
        for f in flags {
            if !self.has_role(*f) {
                return false;
            }
        }
        true
    }

    pub fn has_role(&self, flag: RoleFlag) -> bool {
        self.0 & (flag as u32) != 0
    }
}

impl From<RoleFlag> for RoleMask {
    fn from(flag: RoleFlag) -> Self {
        Self(flag as u32)
    }
}

impl From<u32> for RoleMask {
    fn from(v: u32) -> Self {
        debug_assert!(
            v & !RoleMask::VALID_BITS == 0,
            "u32 value {:#010b} contains invalid role bits (valid bits: {:#010b})",
            v,
            RoleMask::VALID_BITS,
        );
        Self(v)
    }
}

impl From<RoleMask> for u32 {
    fn from(mask: RoleMask) -> Self {
        mask.0
    }
}

/// Entities whose role mask can be read.
///
/// Implemented by [`MemberAggr`](crate::domain::model::aggregate::member::MemberAggr),
/// [`MemberInvitationAggr`](crate::domain::model::aggregate::member_invitation::MemberInvitationAggr),
/// and assignment aggregates.
pub trait RoleView {
    /// Returns the current [`RoleMask`] for this entity.
    fn role_mask(&self) -> RoleMask;
}

/// Entities whose role mask can be set.
pub trait RoleAssign {
    /// Overwrites the role mask with the given value.
    fn assign_roles(&mut self, mask: RoleMask);
}

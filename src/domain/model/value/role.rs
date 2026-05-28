/// Role represents **ONE** specific role that a member or
/// assignment can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Role {
    RawProvider = 1 << 0,
    Translator = 1 << 1,
    Proofreader = 1 << 2,
    Typesetter = 1 << 3,
    Redrawer = 1 << 4,
    Reviewer = 1 << 5,
    Publisher = 1 << 6,
    Admin = 1 << 7,
}

impl Into<u32> for Role {
    fn into(self) -> u32 {
        self as u32
    }
}

/// RoleMask represents a combination of multiple roles that a member or
/// assignment can have. It is implemented as a bitmask, where each bit represents
/// a specific role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleMask(u32);

impl RoleMask {
    pub fn has_any_role(&self, roles: &[Role]) -> bool {
        for role in roles {
            if self.has_role(*role) {
                return true;
            }
        }
        false
    }

    pub fn has_every_role(&self, roles: &[Role]) -> bool {
        for role in roles {
            if !self.has_role(*role) {
                return false;
            }
        }
        true
    }

    fn has_role(&self, role: Role) -> bool {
        self.0 & (role as u32) != 0
    }
}

impl From<Role> for RoleMask {
    fn from(role: Role) -> Self {
        Self(role as u32)
    }
}

impl From<u32> for RoleMask {
    fn from(v: u32) -> Self {
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
/// Implemented by [`Member`](crate::domain::model::aggregate::member::Member),
/// [`MemberInvitation`](crate::domain::model::aggregate::member_invitation::MemberInvitation),
/// and assignment aggregates.
pub trait RoleViewable {
    /// Returns the current [`RoleMask`] for this entity.
    fn roles(&self) -> RoleMask;
}

/// Entities whose role mask can be set.
pub trait RoleAssignable {
    /// Overwrites the role mask with the given value.
    fn assign_roles(&mut self, mask: RoleMask);
}

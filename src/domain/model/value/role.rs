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
/// Implemented by [`MemberAggr`](crate::domain::model::aggr::member::MemberAggr),
/// [`MemberInvitationAggr`](crate::domain::model::aggr::member_invitation::MemberInvitationAggr),
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

#[cfg(test)]
mod tests {
    // has_role_single_bit(RoleMask::has_role)(positive): a single-role mask should report only that role.
    // has_any_role_true(RoleMask::has_any_role)(positive): matching any role should return true when one role is present.
    // has_any_role_false(RoleMask::has_any_role)(negative): matching any role should return false when none are present.
    // has_every_role_true(RoleMask::has_every_role)(positive): matching every role should return true when all roles are present.
    // has_every_role_false_missing_one(RoleMask::has_every_role)(negative): matching every role should return false when one is missing.
    // from_u32_invalid_bits_panics_in_debug(RoleMask::from)(negative): invalid role bits should panic in debug builds.
    // valid_bits_constant(RoleMask::VALID_BITS)(positive): valid role bitmask should cover all defined roles.
    // roundtrip_u32_to_mask_to_u32(RoleMask::from/u32::from)(positive): converting from and back to u32 should preserve valid bits.

    use super::RoleFlag;
    use super::RoleMask;

    #[test]
    fn has_role_single_bit() {
        let mask = RoleMask::from(RoleFlag::Admin);
        assert!(mask.has_role(RoleFlag::Admin));
        assert!(!mask.has_role(RoleFlag::Translator));
    }

    #[test]
    fn has_any_role_true() {
        let mask: RoleMask = RoleFlag::Admin.into();
        assert!(mask.has_any_role(&[RoleFlag::Translator, RoleFlag::Admin]));
    }

    #[test]
    fn has_any_role_false() {
        let mask: RoleMask = RoleFlag::Admin.into();
        assert!(!mask.has_any_role(&[RoleFlag::Translator, RoleFlag::Proofreader]));
    }

    #[test]
    fn has_every_role_true() {
        let mut mask: u32 = 0;
        mask |= Into::<u32>::into(RoleFlag::Translator);
        mask |= Into::<u32>::into(RoleFlag::Proofreader);
        let mask: RoleMask = mask.into();
        assert!(mask.has_every_role(&[RoleFlag::Translator, RoleFlag::Proofreader]));
    }

    #[test]
    fn has_every_role_false_missing_one() {
        let mask: RoleMask = RoleFlag::Translator.into();
        assert!(!mask.has_every_role(&[RoleFlag::Translator, RoleFlag::Proofreader]));
    }

    #[test]
    fn from_u32_invalid_bits_panics_in_debug() {
        let bad: u32 = RoleMask::VALID_BITS << 1;
        let result = std::panic::catch_unwind(|| {
            let _mask: RoleMask = bad.into();
        });
        if cfg!(debug_assertions) {
            assert!(
                result.is_err(),
                "expected panic on invalid bits in debug mode"
            );
        } else {
            assert!(result.is_ok(), "no panic expected in release mode");
        }
    }

    #[test]
    fn valid_bits_constant() {
        assert_eq!(RoleMask::VALID_BITS, (1 << 9) - 1);
    }

    #[test]
    fn roundtrip_u32_to_mask_to_u32() {
        let original: u32 = 0b0000_0000_0010_0101;
        let mask: RoleMask = original.into();
        let back: u32 = mask.into();
        assert_eq!(back, original);
    }
}

// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// /// Role represents **ONE** specific role that a member or
// /// assignment can have.
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// #[repr(u32)]
// pub enum RoleFlag {
//     RawProvider = 1 << 0,
//     Translator = 1 << 1,
//     Proofreader = 1 << 2,
//     Typesetter = 1 << 3,
//     Redrawer = 1 << 4,
//     Reviewer = 1 << 5,
//     Publisher = 1 << 6,
//     Admin = 1 << 7,
//     Assistant = 1 << 8,
// }
// 
// impl From<RoleFlag> for u32 {
//     fn from(val: RoleFlag) -> Self {
//         val as u32
//     }
// }
// 
// impl RoleFlag {
//     /// Tries to interpret a raw `u32` as a single [`RoleFlag`] variant.
//     ///
//     /// Returns `None` if `bits` has zero or more than one bit set.
//     pub fn try_from_single_bit(bits: u32) -> Option<Self> {
//         // Must have exactly one bit set and be within the valid range.
//         if bits == 0 || bits & (bits - 1) != 0 {
//             return None;
//         }
//         // SAFETY: the bit is a valid discriminant because it's a single bit
//         // within the 0..=8 range that matches our repr(u32) enum.
//         if bits > (1 << 8) {
//             return None;
//         }
//         Some(unsafe { std::mem::transmute::<u32, RoleFlag>(bits) })
//     }
// }
// 
// /// RoleMask represents a combination of multiple roles that a member or
// /// assignment can have. It is implemented as a bitmask, where each bit represents
// /// a specific role.
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub struct RoleMask(u32);
// 
// impl RoleMask {
//     /// Bitmask of all valid role bits (bit 0 through 8).
//     const VALID_BITS: u32 = (1 << 9) - 1;
// 
//     pub fn has_any_role(&self, flags: &[RoleFlag]) -> bool {
//         for f in flags {
//             if self.has_role(*f) {
//                 return true;
//             }
//         }
//         false
//     }
// 
//     pub fn has_every_role(&self, flags: &[RoleFlag]) -> bool {
//         for f in flags {
//             if !self.has_role(*f) {
//                 return false;
//             }
//         }
//         true
//     }
// 
//     pub fn has_role(&self, flag: RoleFlag) -> bool {
//         self.0 & (flag as u32) != 0
//     }
// }
// 
// impl From<RoleFlag> for RoleMask {
//     fn from(flag: RoleFlag) -> Self {
//         Self(flag as u32)
//     }
// }
// 
// impl TryFrom<u32> for RoleMask {
//     type Error = ();
// 
//     fn try_from(value: u32) -> Result<Self, Self::Error> {
//         match value {
//             0 => Err(()),
//             bits if bits & !RoleMask::VALID_BITS != 0 => Err(()),
//             bits => Ok(Self(bits)),
//         }
//     }
// }
// 
// impl From<RoleMask> for u32 {
//     fn from(mask: RoleMask) -> Self {
//         mask.0
//     }
// }
// 
// /// Entities whose role mask can be read.
// ///
// /// Implemented by [`MemberAggr`](crate::domain::model::aggr::member::MemberAggr),
// /// [`MemberInvitationAggr`](crate::domain::model::aggr::member_invitation::MemberInvitationAggr),
// /// and assignment aggregates.
// pub trait RoleView {
//     /// Returns the current [`RoleMask`] for this entity.
//     fn role_mask(&self) -> RoleMask;
// }
// 
// /// Entities whose role mask can be set.
// pub trait RoleAssign {
//     /// Overwrites the role mask with the given value.
//     fn assign_roles(&mut self, mask: RoleMask);
// }
// 
// #[cfg(test)]
// mod tests {
//     // has_role_single_bit(RoleMask::has_role)(positive): a single-role mask should report only that role.
//     // has_any_role_true(RoleMask::has_any_role)(positive): matching any role should return true when one role is present.
//     // has_any_role_false(RoleMask::has_any_role)(negative): matching any role should return false when none are present.
//     // has_every_role_true(RoleMask::has_every_role)(positive): matching every role should return true when all roles are present.
//     // has_every_role_false_missing_one(RoleMask::has_every_role)(negative): matching every role should return false when one is missing.
//     // try_from_zero_fails(RoleMask::try_from)(negative): zero should not build a role mask.
//     // try_from_invalid_bits_fails(RoleMask::try_from)(negative): invalid role bits should be rejected.
//     // valid_bits_constant(RoleMask::VALID_BITS)(positive): valid role bitmask should cover all defined roles.
//     // roundtrip_u32_to_mask_to_u32(RoleMask::try_from/u32::from)(positive): converting from and back to u32 should preserve valid bits.
// 
//     use super::*;
// 
//     #[test]
//     fn has_role_single_bit() {
//         let mask = RoleMask::from(RoleFlag::Admin);
//         assert!(mask.has_role(RoleFlag::Admin));
//         assert!(!mask.has_role(RoleFlag::Translator));
//     }
// 
//     #[test]
//     fn has_any_role_true() {
//         let mask: RoleMask = RoleFlag::Admin.into();
//         assert!(mask.has_any_role(&[RoleFlag::Translator, RoleFlag::Admin]));
//     }
// 
//     #[test]
//     fn has_any_role_false() {
//         let mask: RoleMask = RoleFlag::Admin.into();
//         assert!(!mask.has_any_role(&[RoleFlag::Translator, RoleFlag::Proofreader]));
//     }
// 
//     #[test]
//     fn has_every_role_true() {
//         let mut mask: u32 = 0;
//         mask |= Into::<u32>::into(RoleFlag::Translator);
//         mask |= Into::<u32>::into(RoleFlag::Proofreader);
//         let mask = RoleMask::try_from(mask).unwrap();
//         assert!(mask.has_every_role(&[RoleFlag::Translator, RoleFlag::Proofreader]));
//     }
// 
//     #[test]
//     fn has_every_role_false_missing_one() {
//         let mask: RoleMask = RoleFlag::Translator.into();
//         assert!(!mask.has_every_role(&[RoleFlag::Translator, RoleFlag::Proofreader]));
//     }
// 
//     #[test]
//     fn try_from_zero_fails() {
//         let result = RoleMask::try_from(0);
//         assert!(result.is_err());
//     }
// 
//     #[test]
//     fn try_from_invalid_bits_fails() {
//         let bad: u32 = RoleMask::VALID_BITS << 1;
//         let result = RoleMask::try_from(bad);
//         assert!(result.is_err());
//     }
// 
//     #[test]
//     fn valid_bits_constant() {
//         assert_eq!(RoleMask::VALID_BITS, (1 << 9) - 1);
//     }
// 
//     #[test]
//     fn roundtrip_u32_to_mask_to_u32() {
//         let original: u32 = 0b0000_0000_0010_0101;
//         let mask = RoleMask::try_from(original).unwrap();
//         let back: u32 = mask.into();
//         assert_eq!(back, original);
//     }
// }

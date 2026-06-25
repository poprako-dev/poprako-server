//! Newtype wrappers for role-based permission bitmasks.

/// A singular role permission flag represented as a bit position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleBit(pub u32);

/// A composite bitmask combining multiple [RoleBit] flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleMask(pub u32);

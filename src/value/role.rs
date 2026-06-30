//! Newtype wrappers for role-based permission bitmasks.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use poprako_util::i18n::trl;

use crate::result::{ExpectedVariant, RootError, RootResult, accept};

#[cfg(test)]
mod tests;

/// A singular role permission flag represented as a bit position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleField(u32);

impl RoleField {
    /// Raw provider (上传) role.
    pub const RAW_PROVIDER: Self = Self(1 << 0);
    /// Translator (翻译) role.
    pub const TRANSLATOR: Self = Self(1 << 1);
    /// Proofreader (校对) role.
    pub const PROOFREADER: Self = Self(1 << 2);
    /// Typesetter (嵌字) role.
    pub const TYPESETTER: Self = Self(1 << 3);
    /// Redrawer (美工) role.
    pub const REDRAWER: Self = Self(1 << 4);
    /// Reviewer (监修) role.
    pub const REVIEWER: Self = Self(1 << 5);
    /// Publisher (发布) role.
    pub const PUBLISHER: Self = Self(1 << 6);
    /// Admin (管理) role.
    pub const ADMIN: Self = Self(1 << 7);
    /// Bot (机器人) role.
    pub const BOT: Self = Self(1 << 8);

    const VALID_VALUES: &'static [u32] = &[
        1 << 0,
        1 << 1,
        1 << 2,
        1 << 3,
        1 << 4,
        1 << 5,
        1 << 6,
        1 << 7,
        1 << 8,
    ];
}

/// A composite bitmask combining multiple [RoleBit] flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleMask(u32);

impl RoleMask {
    const VALID_BITS: u32 = (1 << 8) - 1;

    /// Check if the mask contains any of the given role bits.
    pub fn has_any_role(&self, bits: &[RoleField]) -> bool {
        bits.iter()
            .any(|role_bit| u32::from(*self) & u32::from(*role_bit) != 0)
    }

    /// Check if the mask contains all of the given role bits.
    pub fn has_every_role(&self, bits: &[RoleField]) -> bool {
        bits.iter()
            .all(|role_bit| u32::from(*self) & u32::from(*role_bit) != 0)
    }

    /// Check if the mask fully contains another mask (all bits set).
    pub fn contains_mask(&self, role_mask: RoleMask) -> bool {
        u32::from(*self) & u32::from(role_mask) == u32::from(role_mask)
    }

    /// Return the union of two masks.
    pub fn union(&self, role_mask: RoleMask) -> RoleMask {
        RoleMask(u32::from(*self) | u32::from(role_mask))
    }
}

/// Convert a raw `u32` to a [`RoleBit`], validating it is a single valid bit.
impl TryFrom<u32> for RoleField {
    type Error = RootError;

    fn try_from(value: u32) -> RootResult<Self> {
        if value == 0 || !Self::VALID_VALUES.contains(&value) || value.count_ones() != 1 {
            return Err(RootError::Expected {
                variant: ExpectedVariant::ArgsInvalid,
                message: trl("error-invalid-role"),
            });
        }

        accept(Self(value))
    }
}

/// Convert a [`RoleBit`] to its underlying `u32` representation.
impl From<RoleField> for u32 {
    fn from(value: RoleField) -> Self {
        value.0
    }
}

/// Serialize a [`RoleBit`] as its raw `u32` value.
impl Serialize for RoleField {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(u32::from(*self))
    }
}

/// Deserialize a [`RoleBit`] from a raw `u32`.
impl<'de> Deserialize<'de> for RoleField {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bits = u32::deserialize(deserializer)?;

        Self::try_from(bits).map_err(|_| D::Error::custom(trl("error-invalid-role")))
    }
}

/// Convert a [`RoleBit`] to a single-bit [`RoleMask`].
impl From<RoleField> for RoleMask {
    fn from(value: RoleField) -> Self {
        Self(u32::from(value))
    }
}

/// Convert a raw `u32` to a [`RoleMask`], validating it contains only valid bits.
impl TryFrom<u32> for RoleMask {
    type Error = RootError;

    fn try_from(value: u32) -> RootResult<Self> {
        if value == 0 || value & !Self::VALID_BITS != 0 {
            return Err(RootError::Expected {
                variant: ExpectedVariant::ArgsInvalid,
                message: trl("error-invalid-role"),
            });
        }

        accept(Self(value))
    }
}

/// Convert a [`RoleMask`] to its underlying `u32` representation.
impl From<RoleMask> for u32 {
    fn from(value: RoleMask) -> Self {
        value.0
    }
}

/// Serialize a [`RoleMask`] as its raw `u32` value.
impl Serialize for RoleMask {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(u32::from(*self))
    }
}

/// Deserialize a [`RoleMask`] from a raw `u32`.
impl<'de> Deserialize<'de> for RoleMask {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bits = u32::deserialize(deserializer)?;

        Self::try_from(bits).map_err(|_| D::Error::custom(trl("error-invalid-role")))
    }
}

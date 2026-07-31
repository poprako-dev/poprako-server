//! Newtype wrappers for role-based permission bitmasks.

use std::result::Result;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::i18n::trl;

use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

// Test fixtures for role conversion, permission, and serde behavior.
#[cfg(test)]
mod tests;

/// A singular role permission flag represented as a bit position.
///
/// Each role is a single bit value:
///
/// | Value | Name          | Description             |
/// |-------|---------------|-------------------------|
/// | 1     | `RAW_PROVIDER` | Raw provider            |
/// | 2     | `TRANSLATOR`   | Translator              |
/// | 4     | `PROOFREADER`  | Proofreader             |
/// | 8     | `TYPESETTER`   | Typesetter              |
/// | 16    | `REDRAWER`     | Redrawer                |
/// | 32    | `REVIEWER`     | Reviewer                |
/// | 64    | `PUBLISHER`    | Publisher               |
/// | 128   | `ADMIN`        | Admin                   |
/// | 256   | `BOT`          | Bot                     |
///
/// Only a **single** valid bit is accepted; composite values are rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[cfg_attr(feature = "swagger", schema(example = 2))]
pub struct RoleField(u32);

impl RoleField {
    /// Raw provider role.
    pub const RAW_PROVIDER: Self = Self(1 << 0);
    /// Translator role.
    pub const TRANSLATOR: Self = Self(1 << 1);
    /// Proofreader role.
    pub const PROOFREADER: Self = Self(1 << 2);
    /// Typesetter role.
    pub const TYPESETTER: Self = Self(1 << 3);
    /// Redrawer role.
    pub const REDRAWER: Self = Self(1 << 4);
    /// Reviewer role.
    pub const REVIEWER: Self = Self(1 << 5);
    /// Publisher role.
    pub const PUBLISHER: Self = Self(1 << 6);
    /// Admin role.
    pub const ADMIN: Self = Self(1 << 7);
    /// Bot role.
    pub const BOT: Self = Self(1 << 8);

    // All valid role flag bit values in canonical order.
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

/// Convert a raw `u32` to a [`RoleBit`], validating it is a single valid bit.
impl TryFrom<u32> for RoleField {
    // Error returned when value does not describe a single valid role bit.
    type Error = BaseError;

    // Validate and convert one raw bit mask into a single role field.
    fn try_from(value: u32) -> BaseRest<Self> {
        //
        if value == 0
            || !Self::VALID_VALUES.contains(&value)
            || value.count_ones() != 1
        {
            let err_message = trl("error-invalid-role");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                raw_value = value,
                "expected error: invalid role field",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        accept(Self(value))
    }
}

impl Serialize for RoleField {
    // Serialize [`RoleField`] as its numeric raw form.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(u32::from(*self))
    }
}

impl<'de> Deserialize<'de> for RoleField {
    // Deserialize a validated [`RoleField`] from raw `u32`.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bits = u32::deserialize(deserializer)?;

        Self::try_from(bits)
            .map_err(|_| D::Error::custom(trl("error-invalid-role")))
    }
}

impl From<RoleField> for u32 {
    // Convert a [`RoleField`] into the underlying `u32` bit value.
    fn from(value: RoleField) -> Self {
        value.0
    }
}

/// A composite bitmask combining multiple role permission flags.
///
/// Bits are OR-ed together from the following role values:
///
/// | Value | Name          | Description             |
/// |-------|---------------|-------------------------|
/// | 1     | `RAW_PROVIDER` | Raw provider            |
/// | 2     | `TRANSLATOR`   | Translator              |
/// | 4     | `PROOFREADER`  | Proofreader             |
/// | 8     | `TYPESETTER`   | Typesetter              |
/// | 16    | `REDRAWER`     | Redrawer                |
/// | 32    | `REVIEWER`     | Reviewer                |
/// | 64    | `PUBLISHER`    | Publisher               |
/// | 128   | `ADMIN`        | Admin                   |
/// | 256   | `BOT`          | Bot                     |
///
/// **Examples:** `1` = RAW_PROVIDER, `6` = TRANSLATOR | PROOFREADER, `255` = all roles except BOT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[cfg_attr(feature = "swagger", schema(example = 34))]
pub struct RoleMask(u32);

impl RoleMask {
    // Bitmask bit-width used by role values in storage and transport.
    const VALID_BITS: u32 = (1 << 8) - 1;

    /// Check if the mask contains any of the given role bits.
    pub fn has_any_role(&self, bits: &[RoleField]) -> bool {
        bits.iter()
            .any(|role| u32::from(*self) & u32::from(*role) != 0)
    }

    /// Check if the mask contains all of the given role bits.
    pub fn has_every_role(&self, bits: &[RoleField]) -> bool {
        bits.iter()
            .all(|role| u32::from(*self) & u32::from(*role) != 0)
    }

    /// Check if the mask fully contains another mask (all bits set).
    pub fn contains_mask(&self, other: RoleMask) -> bool {
        u32::from(*self) & u32::from(other) == u32::from(other)
    }

    /// Return the union of two masks.
    pub fn union(&self, other: RoleMask) -> RoleMask {
        RoleMask(u32::from(*self) | u32::from(other))
    }
}

impl From<RoleField> for RoleMask {
    // Promote a single-role field into a single-bit role mask.
    fn from(value: RoleField) -> Self {
        Self(u32::from(value))
    }
}

impl TryFrom<u32> for RoleMask {
    // Error returned when role-mask bits are empty or contain invalid fields.
    type Error = BaseError;

    // Validate and convert a raw bitmask into a role mask.
    fn try_from(value: u32) -> BaseRest<Self> {
        //
        if value == 0 || value & !Self::VALID_BITS != 0 {
            //
            let err_message = trl("error-invalid-role");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                raw_value = value,
                "expected error: invalid role mask",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        accept(Self(value))
    }
}

impl Serialize for RoleMask {
    // Serialize role masks using raw bitset representation.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(u32::from(*self))
    }
}

impl<'de> Deserialize<'de> for RoleMask {
    // Deserialize a validated role mask from a raw bitmask value.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bits = u32::deserialize(deserializer)?;

        Self::try_from(bits)
            .map_err(|_| D::Error::custom(trl("error-invalid-role")))
    }
}

impl From<RoleMask> for u32 {
    // Convert a mask back into raw `u32` bits.
    fn from(value: RoleMask) -> Self {
        value.0
    }
}

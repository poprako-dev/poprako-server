//! Newtype wrappers for role-based permission bitmasks.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use poprako_util::i18n::trl;

use crate::result::{ExpectedVariant, RootError, RootResult, accept};

#[cfg(test)]
mod tests;

/// A singular role permission flag represented as a bit position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleBit(u32);

impl RoleBit {
    pub const RAW_PROVIDER: Self = Self(1 << 0);
    pub const TRANSLATOR: Self = Self(1 << 1);
    pub const PROOFREADER: Self = Self(1 << 2);
    pub const TYPESETTER: Self = Self(1 << 3);
    pub const REDRAWER: Self = Self(1 << 4);
    pub const REVIEWER: Self = Self(1 << 5);
    pub const PUBLISHER: Self = Self(1 << 6);
    pub const ADMIN: Self = Self(1 << 7);
    pub const BOT: Self = Self(1 << 8);

    const VALID_BITS: u32 = (1 << 9) - 1;
}

/// A composite bitmask combining multiple [RoleBit] flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleMask(u32);

impl RoleMask {
    const VALID_BITS: u32 = (1 << 8) - 1;

    pub fn has_any_role(&self, bits: &[RoleBit]) -> bool {
        bits.iter()
            .any(|role_bit| u32::from(*self) & u32::from(*role_bit) != 0)
    }
}

impl TryFrom<u32> for RoleBit {
    type Error = RootError;

    fn try_from(value: u32) -> RootResult<Self> {
        if value == 0 || value & !Self::VALID_BITS != 0 || value.count_ones() != 1 {
            return Err(RootError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-role"),
            });
        }

        accept(Self(value))
    }
}

impl From<RoleBit> for u32 {
    fn from(value: RoleBit) -> Self {
        value.0
    }
}

impl Serialize for RoleBit {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(u32::from(*self))
    }
}

impl<'de> Deserialize<'de> for RoleBit {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bits = u32::deserialize(deserializer)?;

        Self::try_from(bits).map_err(|_| D::Error::custom(trl("error-invalid-role")))
    }
}

impl From<RoleBit> for RoleMask {
    fn from(value: RoleBit) -> Self {
        Self(u32::from(value))
    }
}

impl TryFrom<u32> for RoleMask {
    type Error = RootError;

    fn try_from(value: u32) -> RootResult<Self> {
        if value == 0 || value & !Self::VALID_BITS != 0 {
            return Err(RootError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-role"),
            });
        }

        accept(Self(value))
    }
}

impl From<RoleMask> for u32 {
    fn from(value: RoleMask) -> Self {
        value.0
    }
}

impl Serialize for RoleMask {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(u32::from(*self))
    }
}

impl<'de> Deserialize<'de> for RoleMask {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bits = u32::deserialize(deserializer)?;

        Self::try_from(bits).map_err(|_| D::Error::custom(trl("error-invalid-role")))
    }
}

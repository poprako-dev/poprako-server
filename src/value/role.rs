//! Newtype wrappers for role-based permission bitmasks.

use poprako_util::i18n::trl;

use crate::result::{ExpectedVariant, RootError, RootResult, accept};

/// A singular role permission flag represented as a bit position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleBit(pub u32);

impl RoleBit {
    pub const RAW_PROVIDER: Self = Self(1 << 0);
    pub const TRANSLATOR: Self = Self(1 << 1);
    pub const PROOFREADER: Self = Self(1 << 2);
    pub const TYPESETTER: Self = Self(1 << 3);
    pub const REDRAWER: Self = Self(1 << 4);
    pub const REVIEWER: Self = Self(1 << 5);
    pub const PUBLISHER: Self = Self(1 << 6);
    pub const ADMIN: Self = Self(1 << 7);
}

/// A composite bitmask combining multiple [RoleBit] flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleMask(pub u32);

impl RoleMask {
    const VALID_BITS: u32 = (1 << 8) - 1;

    pub fn try_from_bits(bits: u32) -> RootResult<Self> {
        if bits == 0 || bits & !Self::VALID_BITS != 0 {
            return Err(RootError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-invalid-role"),
            });
        }

        accept(Self(bits))
    }

    pub fn has_any_role(&self, role_bits: &[RoleBit]) -> bool {
        role_bits.iter().any(|role_bit| self.0 & role_bit.0 != 0)
    }
}

impl From<RoleBit> for RoleMask {
    fn from(value: RoleBit) -> Self {
        Self(value.0)
    }
}

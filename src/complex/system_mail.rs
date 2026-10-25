//! Complex domain logic for [SystemMail] aggregates — ID generation for system-generated notification mails.

use crate::util::next_business_id;
use crate::value::business_id::BusinessIdKind;

/// Generates a unique system mail identifier backed by a snowflake value.
pub fn gen_id() -> String {
    next_business_id(BusinessIdKind::SystemMail)
}

//! Complex domain logic for [SystemMail] aggregates — ID generation for system-generated notification mails.

use crate::util::next_snowflake_id;

/// Domain opers for [`SystemMail`] aggregates: unique identifier generation.
pub struct SystemMailComplex;

impl SystemMailComplex {
    /// Generates a unique system mail identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }
}

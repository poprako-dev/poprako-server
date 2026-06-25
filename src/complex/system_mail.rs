//! Complex domain logic for [SystemMail] aggregates — ID generation for system-generated notification mails.

use uuid::Uuid;

/// Domain operations for [SystemMail] aggregates: unique identifier generation.
pub struct SystemMailComplex;

impl SystemMailComplex {
    /// Generates a unique system mail identifier with a `sys_mail-` prefix using UUID v7.
    pub fn gen_id() -> String {
        format!("sys_mail-{}", Uuid::now_v7())
    }
}

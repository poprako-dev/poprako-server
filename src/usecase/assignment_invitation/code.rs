//! Assignment invitation identifier and code generation helpers.

use crate::util::next_snowflake_id;

/// Generates a snowflake identifier for a new assignment invitation.
pub fn gen_assignment_invitation_id() -> String {
    next_snowflake_id()
}

/// Generates a six-character invitation code derived from a snowflake ID.
pub fn gen_code() -> String {
    //
    let id = next_snowflake_id();

    let skipped_count = id.chars().count().saturating_sub(6);

    id.chars().skip(skipped_count).collect()
}

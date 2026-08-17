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

    let len = id.len();

    match len <= 6 {
        //
        true => id,

        false => id[len - 6..].to_string(),
    }
}

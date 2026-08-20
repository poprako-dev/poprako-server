//! Terminology-base validation, perms, and cascade operations.

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_team_member, check_user_is_team_translator_or_proofreader,
};
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::termbase::TermbaseInfo;
use crate::model::write::termbase::{TermbaseEntry, TermbaseRepl};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

// Trim a termbase name and reject empty values.
fn normalize_name(name: String) -> BaseRest<String> {
    //
    let name = name.trim().to_string();

    if name.is_empty() {
        //
        let err_message = trl("error-termbase-name-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            termbase_name = %name,
            "expected error: termbase name required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    accept(name)
}

// Normalize an optional string by trimming whitespace; empty strings become `None`.
fn normalize_optional(value: Option<String>) -> Option<String> {
    //
    value.and_then(|value| {
        //
        let value = value.trim().to_string();

        match value.is_empty() {
            //
            true => None,

            false => Some(value),
        }
    })
}

/// Pure terminology-base construction and validation helpers.
pub struct TermbaseComplex;

impl TermbaseComplex {
    /// Normalize an optional terminology-base name filter.
    pub fn normalize_fuzzy_name(fuzzy_name: Option<String>) -> Option<String> {
        normalize_optional(fuzzy_name)
    }

    /// Build a validated terminology-base entry.
    pub fn build_entry(
        team_id: Option<String>,
        comic_id: Option<String>,
        name: String,
        description: Option<String>,
        creator_id: String,
    ) -> BaseRest<TermbaseEntry> {
        //
        match (&team_id, &comic_id) {
            //
            (Some(_), None) | (None, Some(_)) => {}

            _ => {
                //
                let err_message = trl("error-invalid-termbase-scope");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    team_id = ?team_id,
                    comic_id = ?comic_id,
                    "expected error: invalid termbase ownership scope",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }
        }

        let (name, description) =
            (normalize_name(name)?, normalize_optional(description));

        accept(TermbaseEntry {
            id: next_snowflake_id(),
            team_id,
            comic_id,
            name,
            description,
            creator_id,
        })
    }

    /// Build a validated terminology-base profile replacement.
    pub fn build_update(
        id: String,
        name: String,
        description: Option<String>,
    ) -> BaseRest<TermbaseRepl> {
        //
        let (name, description) =
            (normalize_name(name)?, normalize_optional(description));

        accept(TermbaseRepl {
            id,
            name,
            description,
        })
    }
}

/// perm checks for terminology-base and terminology-entry resources.
pub struct TermbasePermComplex;

impl TermbasePermComplex {
    /// Verify team membership for a terminology-base read.
    pub fn ensure_user_can_read_team(member_info: &MemberInfo) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }

    /// Verify team membership for terminology bases visible from a comic.
    pub fn ensure_user_can_read_comic(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }

    /// Verify translator or proofreader membership for a terminology-base write.
    pub fn ensure_user_can_write_team(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_translator_or_proofreader(member_info)
    }

    /// Verify team membership for a terminology-base read.
    pub fn ensure_user_can_read(
        member_info: &MemberInfo,
        termbase_info: &TermbaseInfo,
    ) -> BaseRest<()> {
        //
        match (&termbase_info.team_id, &termbase_info.comic_id) {
            //
            (Some(_), None) | (None, Some(_)) => {}

            _ => {
                //
                let err_message = trl("error-invalid-termbase-scope");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    termbase_id = %termbase_info.id,
                    team_id = ?termbase_info.team_id,
                    comic_id = ?termbase_info.comic_id,
                    "expected error: invalid termbase ownership scope",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }
        }

        check_user_is_team_member(member_info)
    }

    /// Verify translator or proofreader membership for a terminology-base write.
    pub fn ensure_user_can_write(
        member_info: &MemberInfo,
        termbase_info: &TermbaseInfo,
    ) -> BaseRest<()> {
        //
        match (&termbase_info.team_id, &termbase_info.comic_id) {
            //
            (Some(_), None) | (None, Some(_)) => {}

            _ => {
                //
                let err_message = trl("error-invalid-termbase-scope");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    termbase_id = %termbase_info.id,
                    team_id = ?termbase_info.team_id,
                    comic_id = ?termbase_info.comic_id,
                    "expected error: invalid termbase ownership scope",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }
        }

        check_user_is_team_translator_or_proofreader(member_info)
    }
}

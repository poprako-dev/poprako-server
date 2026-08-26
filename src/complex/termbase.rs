//! Terminology-base validation, perms, and import planning.

use std::collections::{HashMap, HashSet};

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::term::TermComplex;
use crate::complex::util::{
    check_user_is_team_member, check_user_is_team_translator_or_proofreader,
};
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::term::TermInfo;
use crate::model::read::proj::termbase::TermbaseInfo;
use crate::model::write::term::{
    TermEntry, TermImport, TermRepl, TermUpsertPlan,
};
use crate::model::write::termbase::{
    TermbaseEntry, TermbaseImport, TermbaseRepl,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::termbase::TERMBASE_TERM_LIMIT;

// Build a case-insensitive key after applying persisted uniqueness trimming.
fn normalized_key(value: &str) -> String {
    value.trim().to_lowercase()
}

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

// Merge imported targets after existing values while preserving first spelling and order.
fn merge_targets(
    existing_targets: &[String],
    imported_targets: Vec<String>,
) -> Vec<String> {
    //
    let mut targets = existing_targets.to_vec();

    let mut seen_targets = existing_targets
        .iter()
        .map(|target| normalized_key(target))
        .collect::<HashSet<_>>();

    for target in imported_targets {
        //
        if seen_targets.insert(normalized_key(&target)) {
            targets.push(target);
        }
    }

    targets
}

// Construct the expected capacity error shared by single and imported creates.
fn term_limit_error(
    current_term_count: i32,
    additional_term_count: usize,
) -> BaseError {
    //
    let args =
        HashMap::from([("term_limit".into(), TERMBASE_TERM_LIMIT.into())]);

    let err_message = trl_kv("error-termbase-term-limit", &args);

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        current_term_count,
        additional_term_count,
        term_limit = TERMBASE_TERM_LIMIT,
        "expected error: termbase term limit exceeded",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
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

    /// Normalize and validate a native terminology-base import document.
    pub fn normalize_import(
        termbase_import: TermbaseImport,
    ) -> BaseRest<TermbaseImport> {
        //
        if termbase_import.terms.len() > TERMBASE_TERM_LIMIT as usize {
            return Err(term_limit_error(0, termbase_import.terms.len()));
        }

        let mut seen_sources =
            HashSet::with_capacity(termbase_import.terms.len());

        let mut terms = Vec::with_capacity(termbase_import.terms.len());

        for term_import in termbase_import.terms {
            //
            let term_import = TermComplex::build_import(
                term_import.source,
                term_import.targets,
                term_import.comment,
            )?;

            if !seen_sources.insert(normalized_key(&term_import.source)) {
                //
                let err_message = trl("error-term-source-duplicate");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    source = %term_import.source,
                    "expected error: duplicate imported term source",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }

            terms.push(term_import);
        }

        let (name, description) = (
            normalize_name(termbase_import.name)?,
            normalize_optional(termbase_import.description),
        );

        accept(TermbaseImport {
            name,
            description,
            terms,
        })
    }

    /// Select the imported terminology base's existing same-name target.
    pub fn find_import_target(
        termbase_infos: Vec<TermbaseInfo>,
        name: &str,
    ) -> Option<TermbaseInfo> {
        //
        let name_key = normalized_key(name);

        termbase_infos.into_iter().find(|termbase_info| {
            normalized_key(&termbase_info.name) == name_key
        })
    }

    /// Ensure additional entries keep one terminology base within its limit.
    pub fn ensure_term_capacity(
        current_term_count: i32,
        additional_term_count: usize,
    ) -> BaseRest<()> {
        //
        if current_term_count < 0 {
            //
            tracing::error!(
                current_term_count,
                "unrecoverable error: negative cached termbase term count",
            );

            return Err(BaseError::Unrecoverable {
                message: "negative cached termbase term count".into(),
            });
        }

        let Ok(additional_term_count) = i32::try_from(additional_term_count)
        else {
            return Err(term_limit_error(current_term_count, usize::MAX));
        };

        let Some(resulting_term_count) =
            current_term_count.checked_add(additional_term_count)
        else {
            return Err(term_limit_error(current_term_count, usize::MAX));
        };

        if resulting_term_count > TERMBASE_TERM_LIMIT {
            //
            return Err(term_limit_error(
                current_term_count,
                additional_term_count as usize,
            ));
        }

        accept(())
    }

    /// Build inserts and replacements for a normalized terminology-base import.
    pub fn build_term_upsert_plan(
        termbase_id: &str,
        creator_id: &str,
        current_term_count: i32,
        existing_term_infos: &[TermInfo],
        imported_terms: Vec<TermImport>,
    ) -> BaseRest<TermUpsertPlan> {
        //
        let Ok(loaded_term_count) = i32::try_from(existing_term_infos.len())
        else {
            //
            tracing::error!(
                termbase_id,
                loaded_term_count = existing_term_infos.len(),
                "unrecoverable error: loaded term count overflow",
            );

            return Err(BaseError::Unrecoverable {
                message: "loaded term count overflow".into(),
            });
        };

        if loaded_term_count != current_term_count {
            //
            tracing::error!(
                termbase_id,
                current_term_count,
                loaded_term_count,
                "unrecoverable error: cached termbase term count mismatch",
            );

            return Err(BaseError::Unrecoverable {
                message: "cached termbase term count mismatch".into(),
            });
        }

        let mut entries = Vec::new();

        let mut updates = Vec::new();

        for imported_term in imported_terms {
            //
            let source_key = normalized_key(&imported_term.source);

            let existing_term_info =
                existing_term_infos.iter().find(|term_info| {
                    normalized_key(&term_info.source) == source_key
                });

            match existing_term_info {
                //
                Some(existing_term_info) => {
                    //
                    let targets = merge_targets(
                        &existing_term_info.targets,
                        imported_term.targets,
                    );

                    updates.push(TermRepl {
                        id: existing_term_info.id.clone(),
                        source: imported_term.source,
                        targets,
                        comment: imported_term.comment,
                    });
                }

                None => entries.push(TermEntry {
                    id: next_snowflake_id(),
                    termbase_id: termbase_id.into(),
                    source: imported_term.source,
                    targets: imported_term.targets,
                    comment: imported_term.comment,
                    creator_id: creator_id.into(),
                }),
            }
        }

        Self::ensure_term_capacity(current_term_count, entries.len())?;

        accept(TermUpsertPlan { entries, updates })
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

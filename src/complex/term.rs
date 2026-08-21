//! Terminology-entry construction and validation rules.

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::write::term::{TermEntry, TermImport, TermRepl};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

// Trim a term source and reject empty values after normalization.
fn normalize_source(source: String) -> BaseRest<String> {
    //
    let source = source.trim().to_string();

    if source.is_empty() {
        //
        let err_message = trl("error-term-source-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            source = %source,
            "expected error: term source required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    accept(source)
}

// Normalize all targets and validate required presence, uniqueness and non-empty values.
fn normalize_targets(targets: Vec<String>) -> BaseRest<Vec<String>> {
    //
    if targets.is_empty() {
        //
        let err_message = trl("error-term-targets-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            target_count = targets.len(),
            "expected error: term targets required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    let (mut normalized_targets, mut seen_targets) = (
        Vec::with_capacity(targets.len()),
        HashSet::with_capacity(targets.len()),
    );

    for target in targets {
        //
        let target = target.trim().to_string();

        if target.is_empty() {
            //
            let err_message = trl("error-term-target-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                target = %target,
                target_count = normalized_targets.len(),
                "expected error: term target required",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        if !seen_targets.insert(target.to_lowercase()) {
            //
            let err_message = trl("error-term-target-duplicate");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                target = %target,
                target_count = normalized_targets.len(),
                "expected error: duplicate term target",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        normalized_targets.push(target);
    }

    accept(normalized_targets)
}

// Normalize an optional comment value; empty content becomes `None`.
fn normalize_comment(comment: Option<String>) -> Option<String> {
    //
    comment.and_then(|comment| {
        //
        let comment = comment.trim().to_string();

        match comment.is_empty() {
            //
            true => None,

            false => Some(comment),
        }
    })
}

/// Pure terminology-entry construction and validation helpers.
pub struct TermComplex;

impl TermComplex {
    /// Normalize an optional terminology-entry source filter.
    pub fn normalize_fuzzy_source(
        fuzzy_source: Option<String>,
    ) -> Option<String> {
        normalize_comment(fuzzy_source)
    }

    /// Build a validated terminology entry.
    pub fn build_entry(
        termbase_id: String,
        source: String,
        targets: Vec<String>,
        comment: Option<String>,
        creator_id: String,
    ) -> BaseRest<TermEntry> {
        //
        let (source, targets, comment) = (
            normalize_source(source)?,
            normalize_targets(targets)?,
            normalize_comment(comment),
        );

        accept(TermEntry {
            id: next_snowflake_id(),
            termbase_id,
            source,
            targets,
            comment,
            creator_id,
        })
    }

    /// Build normalized portable terminology-entry content.
    pub fn build_import(
        source: String,
        targets: Vec<String>,
        comment: Option<String>,
    ) -> BaseRest<TermImport> {
        //
        let (source, targets, comment) = (
            normalize_source(source)?,
            normalize_targets(targets)?,
            normalize_comment(comment),
        );

        accept(TermImport {
            source,
            targets,
            comment,
        })
    }

    /// Build a validated terminology-entry replacement.
    pub fn build_update(
        id: String,
        source: String,
        targets: Vec<String>,
        comment: Option<String>,
    ) -> BaseRest<TermRepl> {
        //
        let (source, targets, comment) = (
            normalize_source(source)?,
            normalize_targets(targets)?,
            normalize_comment(comment),
        );

        accept(TermRepl {
            id,
            source,
            targets,
            comment,
        })
    }
}

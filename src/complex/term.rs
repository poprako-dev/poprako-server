//! Terminology-entry construction and validation rules.

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::term::{TermEntry, TermInfoUpdate};
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

// Build a normalized args-level error object using the provided i18n message key.
fn expected(message: &str) -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl(message),
    }
}

// Trim a term source and reject empty values after normalization.
fn normalize_source(source: String) -> BaseResult<String> {
    //
    let source = source.trim().to_string();

    if source.is_empty() {
        return Err(expected("error-term-source-required"));
    }

    accept(source)
}

// Normalize all targets and validate required presence, uniqueness and non-empty values.
fn normalize_targets(targets: Vec<String>) -> BaseResult<Vec<String>> {
    //
    if targets.is_empty() {
        return Err(expected("error-term-targets-required"));
    }

    let mut normalized_targets = Vec::with_capacity(targets.len());

    let mut seen_targets = HashSet::with_capacity(targets.len());

    for target in targets {
        //
        let target = target.trim().to_string();

        if target.is_empty() {
            return Err(expected("error-term-target-required"));
        }

        if !seen_targets.insert(target.to_lowercase()) {
            return Err(expected("error-term-target-duplicate"));
        }

        normalized_targets.push(target);
    }

    accept(normalized_targets)
}

// Normalize an optional comment value; empty content becomes `None`.
fn normalize_comment(comment: Option<String>) -> Option<String> {
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
    ) -> BaseResult<TermEntry> {
        //
        let source = normalize_source(source)?;

        let targets = normalize_targets(targets)?;

        let comment = normalize_comment(comment);

        accept(TermEntry {
            id: next_snowflake_id(),
            termbase_id,
            source,
            targets,
            comment,
            creator_id,
        })
    }

    /// Build a validated terminology-entry replacement.
    pub fn build_update(
        id: String,
        source: String,
        targets: Vec<String>,
        comment: Option<String>,
    ) -> BaseResult<TermInfoUpdate> {
        //
        let source = normalize_source(source)?;

        let targets = normalize_targets(targets)?;

        let comment = normalize_comment(comment);

        accept(TermInfoUpdate {
            id,
            source,
            targets,
            comment,
        })
    }
}

//! Terminology-entry construction and validation rules.

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::write::term::{TermEntry, TermImport, TermRepl};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

/// Borrows a trimmed terminology-entry source filter, omitting blank values.
pub fn normalize_fuzzy_source(fuzzy_source: Option<&str>) -> Option<&str> {
    //
    fuzzy_source
        .map(str::trim)
        .filter(|source| !source.is_empty())
}

/// Build a validated terminology entry.
pub fn build_entry<'a>(
    termbase_id: String,
    source: &'a str,
    targets: Vec<&'a str>,
    comment: Option<&'a str>,
    creator_id: String,
) -> BaseRest<TermEntry<'a>> {
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
pub fn build_import<'a>(
    source: &'a str,
    targets: Vec<&'a str>,
    comment: Option<&'a str>,
) -> BaseRest<TermImport<'a>> {
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
pub fn build_update<'a>(
    id: String,
    source: &'a str,
    targets: Vec<&'a str>,
    comment: Option<&'a str>,
) -> BaseRest<TermRepl<'a>> {
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

// Trim a term source and reject empty values after normalization.
fn normalize_source(source: &str) -> BaseRest<&str> {
    //
    let source = source.trim();

    if source.is_empty() {
        //
        let err_msg = trl("error-term-source-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_msg = %err_msg,
            source = %source,
            "expected error: term source required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            msg: err_msg,
        });
    }

    accept(source)
}

// Normalize all targets and validate required presence, uniqueness and non-empty values.
fn normalize_targets(mut targets: Vec<&str>) -> BaseRest<Vec<&str>> {
    //
    if targets.is_empty() {
        //
        let err_msg = trl("error-term-targets-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_msg = %err_msg,
            target_count = targets.len(),
            "expected error: term targets required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            msg: err_msg,
        });
    }

    let mut seen_targets = HashSet::with_capacity(targets.len());

    for (index, target) in targets.iter_mut().enumerate() {
        //
        *target = target.trim();

        if target.is_empty() {
            //
            let err_msg = trl("error-term-target-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_msg = %err_msg,
                target = %target,
                target_count = index,
                "expected error: term target required",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                msg: err_msg,
            });
        }

        if !seen_targets.insert(target.to_lowercase()) {
            //
            let err_msg = trl("error-term-target-duplicate");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_msg = %err_msg,
                target = %target,
                target_count = index,
                "expected error: duplicate term target",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                msg: err_msg,
            });
        }
    }

    accept(targets)
}

// Normalize an optional comment value; empty content becomes `None`.
fn normalize_comment(comment: Option<&str>) -> Option<&str> {
    comment.map(str::trim).filter(|comment| !comment.is_empty())
}

//! Terminology-entry construction and validation rules.

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::write::term::{TermEntry, TermImport, TermRepl};
use crate::result::{BaseRest, ExpectedVariant};
use crate::util::next_snowflake_id;

/// Borrows a trimmed terminology-entry source filter, omitting blank values.
pub fn normalize_fuzzy_source(fuzzy_source: Option<&str>) -> Option<&str> {
    //
    fuzzy_source
        .map(str::trim)
        .filter(|source| !source.is_empty())
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
    normalize_source(&source)?;

    ensure_targets(&targets)?;

    accept!(TermEntry {
        id: next_snowflake_id(),
        termbase_id,
        source,
        targets,
        comment,
        creator_id,
    })
}

/// Validate portable terminology content while retaining its owned buffers.
pub fn build_import(
    source: String,
    targets: Vec<String>,
    comment: Option<String>,
) -> BaseRest<TermImport> {
    //
    normalize_source(&source)?;

    ensure_targets(&targets)?;

    accept!(TermImport {
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
    normalize_source(&source)?;

    ensure_targets(&targets)?;

    accept!(TermRepl {
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
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-term-source-required");
            source = %source,
            "term source required",
        });
    }

    accept!(source)
}

// Validate targets after trimming, preserving the original owned strings.
fn ensure_targets(targets: &[String]) -> BaseRest<()> {
    //
    if targets.is_empty() {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-term-targets-required");
            target_count = targets.len(),
            "term targets required",
        });
    }

    let mut seen_targets = HashSet::with_capacity(targets.len());

    for (index, target) in targets.iter().enumerate() {
        //
        let target = target.trim();

        if target.is_empty() {
            //
            return Err(expected_error! {
                ExpectedVariant::Args,
                message: trl("error-term-target-required");
                target = %target,
                target_count = index,
                "term target required",
            });
        }

        if !seen_targets.insert(target.to_lowercase()) {
            //
            return Err(expected_error! {
                ExpectedVariant::Args,
                message: trl("error-term-target-duplicate");
                target = %target,
                target_count = index,
                "duplicate term target",
            });
        }
    }

    accept!(())
}

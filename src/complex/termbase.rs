//! Terminology-base validation, perms, and import planning.

/// Pure permission rules.
pub mod perm {
    use poprako_util::i18n::trl;

    use crate::complex::util::{
        check_user_is_team_member, check_user_is_team_translator_or_proofreader,
    };
    use crate::model::read::proj::member::MemberInfo;
    use crate::model::read::proj::termbase::TermbaseInfo;
    use crate::result::{BaseRest, ExpectedVariant};

    /// Verify team membership for a terminology-base read.
    pub const fn ensure_user_can_read_team(
        member_info: &MemberInfo,
    ) -> BaseRest<()> {
        check_user_is_team_member(member_info)
    }

    /// Verify team membership for terminology bases visible from a comic.
    pub const fn ensure_user_can_read_comic(
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
                return Err(expected_error! {
                    ExpectedVariant::Args,
                    message: trl("error-invalid-termbase-scope");
                    termbase_id = %termbase_info.id,
                    team_id = ?termbase_info.team_id,
                    comic_id = ?termbase_info.comic_id,
                    "invalid termbase ownership scope",
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
                return Err(expected_error! {
                    ExpectedVariant::Args,
                    message: trl("error-invalid-termbase-scope");
                    termbase_id = %termbase_info.id,
                    team_id = ?termbase_info.team_id,
                    comic_id = ?termbase_info.comic_id,
                    "invalid termbase ownership scope",
                });
            }
        }

        check_user_is_team_translator_or_proofreader(member_info)
    }
}

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::term as term_complex;
use crate::model::read::proj::term::TermInfo;
use crate::model::read::proj::termbase::TermbaseInfo;
use crate::model::write::term::TermImport;
use crate::model::write::termbase::{
    TermbaseEntry, TermbaseImport, TermbaseRepl,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::shape::write::term::{
    TermAppendPlanShape, TermAppendReplShape, TermEntryShape, TermReplShape,
    TermUpsertPlanShape,
};
use crate::util::next_snowflake_id;
use crate::value::termbase::TERMBASE_TERM_LIMIT;

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
            return Err(expected_error! {
                ExpectedVariant::Args,
                message: trl("error-invalid-termbase-scope");
                team_id = ?team_id,
                comic_id = ?comic_id,
                "invalid termbase ownership scope",
            });
        }
    }

    normalize_name(&name)?;

    accept!(TermbaseEntry {
        id: next_snowflake_id(),
        team_id,
        comic_id,
        name,
        description,
        creator_id,
    })
}

/// Validate a portable batch while preserving its owned text buffers.
pub fn validate_terms(input: Vec<TermImport>) -> BaseRest<Vec<TermImport>> {
    //
    if input.len() > TERMBASE_TERM_LIMIT {
        return Err(term_limit_error(0, input.len()));
    }

    let mut seen_sources = HashSet::with_capacity(input.len());

    let mut terms = Vec::with_capacity(input.len());

    for term_import in input {
        //
        let term_import = term_complex::build_import(
            term_import.source,
            term_import.targets,
            term_import.comment,
        )?;

        if !seen_sources.insert(normalized_key(term_import.trimmed_source())) {
            //
            return Err(expected_error! {
                ExpectedVariant::Args,
                message: trl("error-term-source-duplicate");
                source = %term_import.source,
                "duplicate imported term source",
            });
        }

        terms.push(term_import);
    }

    accept!(terms)
}

/// Validate a native terminology-base import document without copying text.
pub fn validate_import(
    termbase_import: TermbaseImport,
) -> BaseRest<TermbaseImport> {
    //
    normalize_name(&termbase_import.name)?;

    let terms = validate_terms(termbase_import.terms)?;

    let (name, description) =
        (termbase_import.name, termbase_import.description);

    accept!(TermbaseImport {
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

    termbase_infos
        .into_iter()
        .find(|termbase_info| normalized_key(&termbase_info.name) == name_key)
}

/// Ensure additional entries keep one terminology base within its limit.
pub fn ensure_term_capacity(
    current_term_count: usize,
    additional_term_count: usize,
) -> BaseRest<()> {
    //
    let Some(resulting_term_count) =
        current_term_count.checked_add(additional_term_count)
    else {
        return Err(term_limit_error(current_term_count, usize::MAX));
    };

    if resulting_term_count > TERMBASE_TERM_LIMIT {
        //
        return Err(term_limit_error(
            current_term_count,
            additional_term_count,
        ));
    }

    accept!(())
}

/// Build inserts and replacements for a validated terminology-base import.
pub fn build_term_upsert_plan<'a>(
    termbase_id: &'a str,
    creator_id: &'a str,
    current_term_count: usize,
    existing_term_infos: &'a [TermInfo],
    imported_terms: &'a [TermImport],
) -> BaseRest<TermUpsertPlanShape<'a>> {
    //
    let loaded_term_count = existing_term_infos.len();

    if loaded_term_count != current_term_count {
        //
        return Err(unrecoverable_error! {
            message: "cached termbase term count mismatch".into();
            termbase_id,
            current_term_count,
            loaded_term_count,
            "cached termbase term count mismatch",
        });
    }

    let mut entries = Vec::new();

    let mut updates = Vec::new();

    if imported_terms.is_empty() {
        //
        ensure_term_capacity(current_term_count, 0)?;

        return accept!(TermUpsertPlanShape { entries, updates });
    }

    let mut existing_terms_by_source =
        HashMap::with_capacity(loaded_term_count);

    for term_info in existing_term_infos {
        //
        existing_terms_by_source
            .entry(normalized_key(&term_info.source))
            .or_insert(term_info);
    }

    for imported_term in imported_terms {
        //
        let source_key = normalized_key(imported_term.trimmed_source());

        let existing_term_info = existing_terms_by_source.get(&source_key);

        match existing_term_info {
            //
            Some(existing_term_info) => {
                //
                let targets = merge_targets(
                    &existing_term_info.targets,
                    imported_term.trimmed_targets(),
                );

                updates.push(TermReplShape {
                    id: &existing_term_info.id,
                    source: imported_term.trimmed_source(),
                    targets,
                    comment: imported_term.trimmed_comment(),
                });
            }

            None => entries.push(TermEntryShape {
                id: next_snowflake_id(),
                termbase_id,
                source: imported_term.trimmed_source(),
                targets: imported_term.trimmed_targets().collect(),
                comment: imported_term.trimmed_comment(),
                creator_id,
            }),
        }
    }

    ensure_term_capacity(current_term_count, entries.len())?;

    accept!(TermUpsertPlanShape { entries, updates })
}

/// Build a validated terminology-base profile replacement.
pub fn build_update(
    id: String,
    name: String,
    description: Option<String>,
) -> BaseRest<TermbaseRepl> {
    //
    normalize_name(&name)?;

    accept!(TermbaseRepl {
        id,
        name,
        description,
    })
}

/// Plan atomic append writes while retaining identity and skipping unchanged entries.
pub fn build_term_append_plan<'a>(
    termbase_id: &'a str,
    creator_id: &'a str,
    current_term_count: usize,
    existing_term_infos: &'a [TermInfo],
    incoming_terms: &'a [TermImport],
) -> BaseRest<TermAppendPlanShape<'a>> {
    //
    let term_upsert_plan = build_term_upsert_plan(
        termbase_id,
        creator_id,
        current_term_count,
        existing_term_infos,
        incoming_terms,
    )?;

    let existing_terms = existing_term_infos
        .iter()
        .map(|term| (term.id.as_str(), term))
        .collect::<HashMap<_, _>>();

    let mut updates = Vec::new();

    for update in term_upsert_plan.updates {
        // Every replacement was resolved from these persisted entries by the shared planner.
        let Some(existing) = existing_terms.get(update.id) else {
            //
            return Err(unrecoverable_error! {
                message: "append replacement has no existing term".into();
                term_id = %update.id,
                "append replacement has no existing term",
            });
        };

        let comment =
            merge_append_comment(existing.comment.as_deref(), update.comment);

        if update.targets == existing.targets && comment == existing.comment {
            continue;
        }

        updates.push(TermAppendReplShape {
            id: update.id,
            source: &existing.source,
            targets: update.targets,
            comment,
        });
    }

    accept!(TermAppendPlanShape {
        entries: term_upsert_plan.entries,
        updates,
    })
}

// Trim a termbase name and reject empty values.
fn normalize_name(name: &str) -> BaseRest<&str> {
    //
    let name = name.trim();

    if name.is_empty() {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-termbase-name-required");
            termbase_name = %name,
            "termbase name required",
        });
    }

    accept!(name)
}

// Construct the expected capacity error shared by single and imported creates.
fn term_limit_error(
    current_term_count: usize,
    additional_term_count: usize,
) -> BaseError {
    //
    let args =
        HashMap::from([("term_limit".into(), TERMBASE_TERM_LIMIT.into())]);

    expected_error! {
        ExpectedVariant::Args,
        message: trl_kv("error-termbase-term-limit", &args);
        current_term_count,
        additional_term_count,
        term_limit = TERMBASE_TERM_LIMIT,
        "termbase term limit exceeded",
    }
}

// Build a case-insensitive key after applying persisted uniqueness trimming.
fn normalized_key(value: &str) -> String {
    value.trim().to_lowercase()
}

// Merge imported targets after existing values while preserving first spelling and order.
fn merge_targets<'a, I>(
    existing_targets: &'a [String],
    imported_targets: I,
) -> Vec<&'a str>
where
    I: Iterator<Item = &'a str>,
{
    //
    let mut targets = existing_targets
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let mut seen_targets = existing_targets
        .iter()
        .map(|target| normalized_key(target))
        .collect::<HashSet<_>>();

    for target in imported_targets {
        //
        if seen_targets.insert(normalized_key(target)) {
            targets.push(target);
        }
    }

    targets
}

// Compare comments as whole blocks, preserving absent, equal, and multiline text.
fn merge_append_comment(
    existing: Option<&str>,
    incoming: Option<&str>,
) -> Option<String> {
    //
    match (existing, incoming) {
        //
        (existing, None) => existing.map(str::to_owned),

        (None, incoming) => incoming.map(str::to_owned),

        (Some(existing), Some(incoming)) if existing == incoming => {
            Some(existing.into())
        }

        (Some(existing), Some(incoming)) => {
            Some([existing, incoming].join("\n"))
        }
    }
}

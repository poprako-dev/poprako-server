//! Batched terminology import persistence.

use diesel::pg::Pg;
use diesel::prelude::{
    BoxableExpression, ExpressionMethods as _, QueryDsl as _,
};
use diesel::sql_types::{Array, Nullable, Text};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;

use crate::part_impl::repo::rdb_impl::entity::term::TermItem;
use crate::part_impl::repo::rdb_impl::schema::t_term::dsl::{
    f_comment, f_id, f_source, f_targets, f_termbase_id, f_updated_at, t_term,
};
use crate::result::{BaseRest, accept};
use crate::shape::write::term::{TermEntryShape, TermReplShape};
use crate::shared::result::diesel;

// Boxed Diesel expressions used to build typed Term batch updates.
type TermExpr<'a, SqlType> =
    Box<dyn BoxableExpression<t_term, Pg, SqlType = SqlType> + 'a>;

/// Applies bounded imported inserts and replacements inside one adapter step.
#[instrument(level = "info", skip_all)]
pub async fn upsert_terms(
    conn: &mut RdbConn,
    termbase_id: &str,
    entries: &[TermEntryShape<'_>],
    updates: &[TermReplShape<'_>],
) -> BaseRest<()> {
    //
    ensure_upsert_ids(termbase_id, entries, updates)?;

    if !entries.is_empty() {
        //
        let items = entries.iter().map(TermItem::from).collect::<Vec<_>>();

        let affected = diesel::insert_into(t_term)
            .values(&items)
            .execute(conn)
            .await
            .map_err(diesel)?;

        ensure_upsert_affected("insert", affected, entries.len())?;
    }

    if !updates.is_empty() {
        //
        let ids = updates.iter().map(|update| update.id).collect::<Vec<_>>();

        let source_case =
            text_case(updates, Box::new(f_source), |update| update.source);

        let comment_case =
            nullable_text_case(updates, Box::new(f_comment), |update| {
                update.comment
            });

        let targets_case = targets_case(updates);

        let updated_at = OffsetDateTime::now_utc();

        let affected = diesel::update(
            t_term
                .filter(f_termbase_id.eq(termbase_id))
                .filter(f_id.eq_any(ids)),
        )
        .set((
            f_source.eq(source_case),
            f_targets.eq(targets_case),
            f_comment.eq(comment_case),
            f_updated_at.eq(updated_at),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

        ensure_upsert_affected("update", affected, updates.len())?;
    }

    accept(())
}

// Validates Termbase scope and uniqueness before executing a batch.
fn ensure_upsert_ids(
    termbase_id: &str,
    entries: &[TermEntryShape<'_>],
    updates: &[TermReplShape<'_>],
) -> BaseRest<()> {
    //
    let entries_in_scope =
        entries.iter().all(|entry| entry.termbase_id == termbase_id);

    let mut ids = std::collections::HashSet::with_capacity(
        entries.len().saturating_add(updates.len()),
    );

    let ids_unique = entries
        .iter()
        .map(|entry| entry.id.as_str())
        .chain(updates.iter().map(|update| update.id))
        .all(|id| ids.insert(id));

    if entries_in_scope && ids_unique {
        return accept(());
    }

    Err(unrecoverable_error! {
        message: "invalid Term upsert scope or duplicate id".into();
        termbase_id,
        entries_in_scope,
        ids_unique,
        "invalid Term upsert scope or duplicate id",
    })
}

// Builds a typed CASE expression for text replacements.
// Verifies that a batch operation changed exactly the requested items.
fn ensure_upsert_affected(
    operation: &str,
    affected: usize,
    expected: usize,
) -> BaseRest<()> {
    //
    if affected == expected {
        return accept(());
    }

    Err(unrecoverable_error! {
        message: "Term batch affected an unexpected row count".into();
        operation,
        affected,
        expected,
        "Term batch affected an unexpected row count",
    })
}

// Builds a typed CASE expression for text replacements.
fn text_case<'a, F>(
    updates: &'a [TermReplShape<'a>],
    mut expression: TermExpr<'a, Text>,
    field: F,
) -> TermExpr<'a, Text>
where
    F: Fn(&'a TermReplShape<'a>) -> &'a str,
{
    for update in updates {
        //
        expression = Box::new(
            diesel::dsl::case_when::<_, _, Text>(
                f_id.eq(update.id),
                field(update),
            )
            .otherwise(expression),
        );
    }

    expression
}

// Builds a typed CASE expression for nullable text replacements.
fn nullable_text_case<'a, F>(
    updates: &'a [TermReplShape<'a>],
    mut expression: TermExpr<'a, Nullable<Text>>,
    field: F,
) -> TermExpr<'a, Nullable<Text>>
where
    F: Fn(&'a TermReplShape<'a>) -> Option<&'a str>,
{
    for update in updates {
        //
        expression = Box::new(
            diesel::dsl::case_when::<_, _, Nullable<Text>>(
                f_id.eq(update.id),
                field(update),
            )
            .otherwise(expression),
        );
    }

    expression
}

// Builds a typed CASE expression for target-array replacements.
fn targets_case<'a>(
    updates: &'a [TermReplShape<'a>],
) -> TermExpr<'a, Array<Nullable<Text>>> {
    //
    let mut expression =
        Box::new(f_targets) as TermExpr<'a, Array<Nullable<Text>>>;

    for update in updates {
        //
        let targets = update
            .targets
            .iter()
            .map(|target| Some(*target))
            .collect::<Vec<_>>();

        expression = Box::new(
            diesel::dsl::case_when::<_, _, Array<Nullable<Text>>>(
                f_id.eq(update.id),
                targets,
            )
            .otherwise(expression),
        );
    }

    expression
}

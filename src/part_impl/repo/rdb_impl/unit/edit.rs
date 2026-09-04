//! RDB-backed Unit edit application and sequence mutation.

use std::collections::BTreeMap;

use diesel::expression::functions::declare_sql_function;
use diesel::expression_methods::NullableExpressionMethods as _;
use diesel::pg::Pg;
use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel::sql_types::{Bool, Double, Nullable, Text, Timestamptz};
use diesel::{
    AggregateExpressionMethods as _, BoolExpressionMethods as _,
    BoxableExpression,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;

use crate::complex::unit::UnitComplex;
use crate::model::read::proj::unit::{UnitCountMetrics, UnitOrder};
use crate::model::write::unit::UnitEdit;
use crate::part_impl::repo::rdb_impl::entity::unit::{
    UnitAspectRow, UnitEntryRow,
};
use crate::part_impl::repo::rdb_impl::numeric::usize_from_i64;
use crate::part_impl::repo::rdb_impl::schema::t_unit;
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_hidden_at, f_id, f_is_bubble, f_is_proofread, f_last_proofreader_id,
    f_last_translator_id, f_next_id, f_page_id, f_proofread_text,
    f_translated_text, f_updated_at, f_x_coord, f_y_coord,
    t_unit as unit_table,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;

#[declare_sql_function]
extern "SQL" {
    /// `PostgreSQL` text trim with an explicit character set.
    // PostgreSQL text trim with an explicit character set.
    fn btrim(string: Nullable<Text>, characters: Text) -> Nullable<Text>;
}

// `char::is_whitespace` code points accepted by Unit text semantics.
const UNIT_TEXT_WHITESPACE: &str = "\u{0009}\u{000A}\u{000B}\u{000C}\u{000D}\u{0020}\u{0085}\u{00A0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}";

// A boxed, table-scoped Diesel expression used to build typed CASE trees.
type UnitExpr<'a, SqlType> =
    Box<dyn BoxableExpression<t_unit::table, Pg, SqlType = SqlType> + 'a>;

// Existing Unit mutations keyed in deterministic ID order.
type UnitChanges<'a> = BTreeMap<&'a str, UnitAspectRow<'a>>;

/// Applies normalized Unit edits and returns the latest visible counters.
#[instrument(level = "info", skip_all)]
pub async fn apply_edits(
    conn: &mut RdbConn,
    page_id: &str,
    orders: &[UnitOrder],
    edits: &[UnitEdit],
) -> BaseRest<UnitCountMetrics> {
    //
    let edit_plan = UnitComplex::plan_edit_sequence(orders, edits)?;

    let mut create_entries = Vec::new();

    for edit in edits {
        //
        let UnitEdit::Create { id, .. } = edit else {
            continue;
        };

        let next_id = edit_plan.next_id(id)?;

        let Some(create_entry) =
            UnitEntryRow::from_edit(page_id, edit, next_id)
        else {
            return Err(invalid_unit_edit_plan_err());
        };

        create_entries.push(create_entry);
    }

    apply_create_rows(conn, page_id, &create_entries).await?;

    let mut changes = UnitChanges::new();

    for edit in edits {
        //
        let (id, change) = match edit {
            //
            UnitEdit::Save { id, .. } => {
                //
                let next_id = edit_plan.next_id(id)?;

                (
                    id.as_str(),
                    UnitAspectRow::new().order(next_id).apply_edit(edit),
                )
            }

            UnitEdit::Delete { id } => {
                (id.as_str(), UnitAspectRow::new().hide())
            }

            UnitEdit::Create { .. } => continue,
        };

        if changes.insert(id, change).is_some() {
            return Err(invalid_unit_edit_plan_err());
        }
    }

    for successor_change in edit_plan.changed_successors() {
        //
        if let Some(change) = changes.get_mut(successor_change.id()) {
            //
            change.f_next_id = Some(successor_change.next_id());

            continue;
        }

        let change = UnitAspectRow::new().order(successor_change.next_id());

        changes.insert(successor_change.id(), change);
    }

    apply_update_rows(conn, page_id, &changes).await?;

    count_visible_units(conn, page_id).await
}

// Reports an impossible mismatch between normalized edits and their plan.
fn invalid_unit_edit_plan_err() -> BaseError {
    //
    tracing::error!(
        "unrecoverable error: normalized Unit edits disagree with edit plan"
    );

    BaseError::Unrecoverable {
        message: "normalized Unit edits disagree with edit plan".into(),
    }
}

// Insert an exact Unit row set in one statement.
async fn apply_create_rows(
    conn: &mut RdbConn,
    page_id: &str,
    entries: &[UnitEntryRow<'_>],
) -> BaseRest<()> {
    //
    if entries.is_empty() {
        return accept(());
    }

    let affected = diesel::insert_into(unit_table)
        .values(entries)
        .execute(conn)
        .await
        .map_err(diesel)?;

    ensure_affected(page_id, "batch", "create", affected, entries.len())
}

// Applies every existing-row mutation through one typed CASE update.
async fn apply_update_rows(
    conn: &mut RdbConn,
    page_id: &str,
    changes: &UnitChanges<'_>,
) -> BaseRest<()> {
    //
    if changes.is_empty() {
        return accept(());
    }

    let ids = changes.keys().copied().collect::<Vec<_>>();

    let next_id_case =
        nullable_text_case(changes, Box::new(f_next_id), |change| {
            change.f_next_id
        });

    let hidden_at_case =
        nullable_timestamp_case(changes, Box::new(f_hidden_at), |change| {
            change.f_hidden_at
        });

    let is_bubble_case =
        bool_case(changes, Box::new(f_is_bubble), |change| change.f_is_bubble);

    let is_proofread_case =
        bool_case(changes, Box::new(f_is_proofread), |change| {
            change.f_is_proofread
        });

    let x_coord_case =
        double_case(changes, Box::new(f_x_coord), |change| change.f_x_coord);

    let y_coord_case =
        double_case(changes, Box::new(f_y_coord), |change| change.f_y_coord);

    let translated_text_case =
        nullable_text_case(changes, Box::new(f_translated_text), |change| {
            change.f_translated_text
        });

    let translator_id_case =
        nullable_text_case(changes, Box::new(f_last_translator_id), |change| {
            change.f_last_translator_id
        });

    let proofread_text_case =
        nullable_text_case(changes, Box::new(f_proofread_text), |change| {
            change.f_proofread_text
        });

    let proofreader_id_case = nullable_text_case(
        changes,
        Box::new(f_last_proofreader_id),
        |change| change.f_last_proofreader_id,
    );

    let updated_at = OffsetDateTime::now_utc();

    let affected = diesel::update(
        unit_table
            .filter(f_page_id.eq(page_id))
            .filter(f_id.eq_any(&ids)),
    )
    .set((
        f_next_id.eq(next_id_case),
        f_hidden_at.eq(hidden_at_case),
        f_is_bubble.eq(is_bubble_case),
        f_is_proofread.eq(is_proofread_case),
        f_x_coord.eq(x_coord_case),
        f_y_coord.eq(y_coord_case),
        f_translated_text.eq(translated_text_case),
        f_last_translator_id.eq(translator_id_case),
        f_proofread_text.eq(proofread_text_case),
        f_last_proofreader_id.eq(proofreader_id_case),
        f_updated_at.eq(updated_at),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    ensure_affected(page_id, "batch", "update", affected, changes.len())
}

// Aggregates visible Unit counters inside PostgreSQL.
async fn count_visible_units(
    conn: &mut RdbConn,
    page_id: &str,
) -> BaseRest<UnitCountMetrics> {
    //
    let has_translation = btrim(f_translated_text, UNIT_TEXT_WHITESPACE).ne("");

    let has_revision = btrim(f_proofread_text, UNIT_TEXT_WHITESPACE).ne("");

    let (total, translated, proofread) = unit_table
        .filter(f_page_id.eq(page_id))
        .filter(f_hidden_at.is_null())
        .select((
            diesel::dsl::count_star(),
            diesel::dsl::count(f_id).aggregate_filter(
                has_translation.or(has_revision).assume_not_null(),
            ),
            diesel::dsl::count(f_id).aggregate_filter(f_is_proofread),
        ))
        .get_result::<(i64, i64, i64)>(conn)
        .await
        .map_err(diesel)?;

    accept(UnitCountMetrics {
        total: usize_from_i64(total, "count visible Units")?,
        translated: usize_from_i64(
            translated,
            "count translated visible Units",
        )?,
        proofread: usize_from_i64(proofread, "count proofread visible Units")?,
    })
}

// Reject an invariant-breaking affected-row count.
fn ensure_affected(
    page_id: &str,
    unit_id: &str,
    operation: &str,
    affected: usize,
    expected: usize,
) -> BaseRest<()> {
    //
    if affected == expected {
        return accept(());
    }

    tracing::error!(
        page_id,
        unit_id,
        operation,
        affected,
        expected,
        stage = "apply_edits",
        "unrecoverable error: Unit batch affected an unexpected row count",
    );

    Err(BaseError::Unrecoverable {
        message: "Unit batch affected an unexpected row count".into(),
    })
}

// Builds a typed nullable-text CASE expression for one column.
fn nullable_text_case<'a, F>(
    changes: &'a UnitChanges<'a>,
    mut expr: UnitExpr<'a, Nullable<Text>>,
    field: F,
) -> UnitExpr<'a, Nullable<Text>>
where
    F: Fn(&UnitAspectRow<'a>) -> Option<Option<&'a str>>,
{
    for (id, change) in changes {
        //
        let Some(value) = field(change) else {
            continue;
        };

        expr = Box::new(
            diesel::dsl::case_when::<_, _, Nullable<Text>>(f_id.eq(*id), value)
                .otherwise(expr),
        );
    }

    expr
}

// Builds a typed nullable-timestamp CASE expression for one column.
fn nullable_timestamp_case<'a, F>(
    changes: &'a UnitChanges<'a>,
    mut expr: UnitExpr<'a, Nullable<Timestamptz>>,
    field: F,
) -> UnitExpr<'a, Nullable<Timestamptz>>
where
    F: Fn(&UnitAspectRow<'a>) -> Option<Option<OffsetDateTime>>,
{
    for (id, change) in changes {
        //
        let Some(value) = field(change) else {
            continue;
        };

        expr = Box::new(
            diesel::dsl::case_when::<_, _, Nullable<Timestamptz>>(
                f_id.eq(*id),
                value,
            )
            .otherwise(expr),
        );
    }

    expr
}

// Builds a typed boolean CASE expression for one column.
fn bool_case<'a, F>(
    changes: &'a UnitChanges<'a>,
    mut expr: UnitExpr<'a, Bool>,
    field: F,
) -> UnitExpr<'a, Bool>
where
    F: Fn(&UnitAspectRow<'a>) -> Option<bool>,
{
    for (id, change) in changes {
        //
        let Some(value) = field(change) else {
            continue;
        };

        expr = Box::new(
            diesel::dsl::case_when::<_, _, Bool>(f_id.eq(*id), value)
                .otherwise(expr),
        );
    }

    expr
}

// Builds a typed double-precision CASE expression for one column.
fn double_case<'a, F>(
    changes: &'a UnitChanges<'a>,
    mut expr: UnitExpr<'a, Double>,
    field: F,
) -> UnitExpr<'a, Double>
where
    F: Fn(&UnitAspectRow<'a>) -> Option<f64>,
{
    for (id, change) in changes {
        //
        let Some(value) = field(change) else {
            continue;
        };

        expr = Box::new(
            diesel::dsl::case_when::<_, _, Double>(f_id.eq(*id), value)
                .otherwise(expr),
        );
    }

    expr
}

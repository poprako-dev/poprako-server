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

use crate::complex::unit_save as unit_save_complex;
use crate::model::read::proj::unit::{UnitCountMetrics, UnitOrder};
use crate::model::write::unit::UnitEdit;
use crate::part_impl::repo::rdb_impl::entity::unit::{UnitItem, UnitPack};
use crate::part_impl::repo::rdb_impl::numeric::usize_from_i64;
use crate::part_impl::repo::rdb_impl::schema::t_unit;
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_hidden_at, f_id, f_is_bubble, f_is_flagged, f_is_proofread,
    f_last_proofreader_id, f_last_translator_id, f_next_id, f_page_id,
    f_proofread_text, f_translated_text, f_translation_note, f_typography_note,
    f_updated_at, f_x_coord, f_y_coord, t_unit as unit_table,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;
use crate::util::Patch;

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
type UnitPacks<'a> = BTreeMap<&'a str, UnitPack<'a>>;

/// Applies normalized Unit edits and returns the latest visible counters.
#[instrument(level = "info", skip_all)]
pub async fn apply_edits(
    conn: &mut RdbConn,
    page_id: &str,
    orders: &[UnitOrder],
    edits: &[UnitEdit],
) -> BaseRest<UnitCountMetrics> {
    //
    let write_plan = unit_save_complex::plan_writes(orders, edits)?;

    if !write_plan.skipped_ids.is_empty() {
        //
        let matched = unit_table
            .filter(f_page_id.eq(page_id))
            .filter(f_id.eq_any(&write_plan.skipped_ids))
            .count()
            .get_result::<i64>(conn)
            .await
            .map_err(diesel)?;

        ensure_affected(
            page_id,
            "batch",
            "validate skipped targets",
            usize_from_i64(matched, "count skipped Unit targets")?,
            write_plan.skipped_ids.len(),
        )?;
    }

    let mut items = Vec::new();

    for edit in &write_plan.creates {
        //
        let UnitEdit::Create { next_id, .. } = edit else {
            return Err(invalid_unit_edit_plan_error());
        };

        let item = UnitItem::from_edit(page_id, edit, next_id.as_deref())
            .ok_or_else(invalid_unit_edit_plan_error)?;

        items.push(item);
    }

    apply_create_items(conn, page_id, &items).await?;

    let mut packs = UnitPacks::new();

    for (id, mutation) in &write_plan.changes {
        //
        let mut pack = UnitPack::new();

        if let Some(edit) = mutation.edit {
            pack = pack.apply_edit(edit);
        }

        pack.f_next_id = match &mutation.next_id {
            //
            Patch::Skip => None,

            Patch::Clear => Some(None),

            Patch::Assign { value } => Some(Some(*value)),
        };

        pack.f_hidden_at = mutation
            .is_hidden
            .map(|hidden| hidden.then(OffsetDateTime::now_utc));

        packs.insert(*id, pack);
    }

    apply_update_packs(conn, page_id, &packs).await?;

    count_visible_units(conn, page_id).await
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

    Err(unrecoverable_error! {
        message: "Unit batch affected an unexpected row count".into();
        page_id,
        unit_id,
        operation,
        affected,
        expected,
        stage = "apply_edits",
        "Unit batch affected an unexpected row count",
    })
}

// Reports an impossible mismatch between normalized edits and their plan.
fn invalid_unit_edit_plan_error() -> BaseError {
    //
    unrecoverable_error! {
        message: "normalized Unit edits disagree with edit plan".into();
        "normalized Unit edits disagree with edit plan",
    }
}

// Insert an exact Unit row set in one statement.
async fn apply_create_items(
    conn: &mut RdbConn,
    page_id: &str,
    items: &[UnitItem<'_>],
) -> BaseRest<()> {
    //
    if items.is_empty() {
        return accept(());
    }

    let affected = diesel::insert_into(unit_table)
        .values(items)
        .execute(conn)
        .await
        .map_err(diesel)?;

    ensure_affected(page_id, "batch", "create", affected, items.len())
}

// Applies every existing-row mutation through one typed CASE update.
async fn apply_update_packs(
    conn: &mut RdbConn,
    page_id: &str,
    packs: &UnitPacks<'_>,
) -> BaseRest<()> {
    //
    if packs.is_empty() {
        return accept(());
    }

    let ids = packs.keys().copied().collect::<Vec<_>>();

    let next_id_case =
        nullable_text_case(packs, Box::new(f_next_id), |pack| pack.f_next_id);

    let hidden_at_case =
        nullable_timestamp_case(packs, Box::new(f_hidden_at), |pack| {
            pack.f_hidden_at
        });

    let is_bubble_case =
        bool_case(packs, Box::new(f_is_bubble), |pack| pack.f_is_bubble);

    let is_flagged_case =
        bool_case(packs, Box::new(f_is_flagged), |pack| pack.f_is_flagged);

    let is_proofread_case =
        bool_case(packs, Box::new(f_is_proofread), |pack| pack.f_is_proofread);

    let x_coord_case =
        double_case(packs, Box::new(f_x_coord), |pack| pack.f_x_coord);

    let y_coord_case =
        double_case(packs, Box::new(f_y_coord), |pack| pack.f_y_coord);

    let translated_text_case =
        nullable_text_case(packs, Box::new(f_translated_text), |pack| {
            pack.f_translated_text
        });

    let translator_id_case =
        nullable_text_case(packs, Box::new(f_last_translator_id), |pack| {
            pack.f_last_translator_id
        });

    let proofread_text_case =
        nullable_text_case(packs, Box::new(f_proofread_text), |pack| {
            pack.f_proofread_text
        });

    let proofreader_id_case =
        nullable_text_case(packs, Box::new(f_last_proofreader_id), |pack| {
            pack.f_last_proofreader_id
        });

    let typography_note_case =
        nullable_text_case(packs, Box::new(f_typography_note), |pack| {
            pack.f_typography_note
        });

    let translation_note_case =
        nullable_text_case(packs, Box::new(f_translation_note), |pack| {
            pack.f_translation_note
        });

    let updated_at = OffsetDateTime::now_utc();

    let affected = diesel::update(
        unit_table
            .filter(f_page_id.eq(page_id))
            .filter(f_id.eq_any(&ids)),
    )
    .set((
        next_id_case.map(|expr| f_next_id.eq(expr)),
        hidden_at_case.map(|expr| f_hidden_at.eq(expr)),
        is_bubble_case.map(|expr| f_is_bubble.eq(expr)),
        is_flagged_case.map(|expr| f_is_flagged.eq(expr)),
        is_proofread_case.map(|expr| f_is_proofread.eq(expr)),
        x_coord_case.map(|expr| f_x_coord.eq(expr)),
        y_coord_case.map(|expr| f_y_coord.eq(expr)),
        translated_text_case.map(|expr| f_translated_text.eq(expr)),
        translator_id_case.map(|expr| f_last_translator_id.eq(expr)),
        proofread_text_case.map(|expr| f_proofread_text.eq(expr)),
        proofreader_id_case.map(|expr| f_last_proofreader_id.eq(expr)),
        typography_note_case.map(|expr| f_typography_note.eq(expr)),
        translation_note_case.map(|expr| f_translation_note.eq(expr)),
        f_updated_at.eq(updated_at),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    ensure_affected(page_id, "batch", "update", affected, packs.len())
}

// Aggregates visible Unit counters inside PostgreSQL.
async fn count_visible_units(
    conn: &mut RdbConn,
    page_id: &str,
) -> BaseRest<UnitCountMetrics> {
    //
    let has_translation = btrim(f_translated_text, UNIT_TEXT_WHITESPACE).ne("");

    let has_revision = btrim(f_proofread_text, UNIT_TEXT_WHITESPACE).ne("");

    let has_typography_note =
        btrim(f_typography_note, UNIT_TEXT_WHITESPACE).ne("");

    let (total, translated, proofread) = unit_table
        .filter(f_page_id.eq(page_id))
        .filter(f_hidden_at.is_null())
        .select((
            diesel::dsl::count_star(),
            diesel::dsl::count(f_id).aggregate_filter(
                has_translation
                    .or(has_revision)
                    .or(has_typography_note)
                    .assume_not_null(),
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

// Builds a typed nullable-text CASE expression for one column.
fn nullable_text_case<'a, F>(
    packs: &'a UnitPacks<'a>,
    mut expr: UnitExpr<'a, Nullable<Text>>,
    field: F,
) -> Option<UnitExpr<'a, Nullable<Text>>>
where
    F: Fn(&UnitPack<'a>) -> Option<Option<&'a str>>,
{
    let mut has_values = false;

    for (id, pack) in packs {
        //
        let Some(value) = field(pack) else {
            continue;
        };

        has_values = true;

        expr = Box::new(
            diesel::dsl::case_when::<_, _, Nullable<Text>>(f_id.eq(*id), value)
                .otherwise(expr),
        );
    }

    has_values.then_some(expr)
}

// Builds a typed nullable-timestamp CASE expression for one column.
fn nullable_timestamp_case<'a, F>(
    packs: &'a UnitPacks<'a>,
    mut expr: UnitExpr<'a, Nullable<Timestamptz>>,
    field: F,
) -> Option<UnitExpr<'a, Nullable<Timestamptz>>>
where
    F: Fn(&UnitPack<'a>) -> Option<Option<OffsetDateTime>>,
{
    let mut has_values = false;

    for (id, pack) in packs {
        //
        let Some(value) = field(pack) else {
            continue;
        };

        has_values = true;

        expr = Box::new(
            diesel::dsl::case_when::<_, _, Nullable<Timestamptz>>(
                f_id.eq(*id),
                value,
            )
            .otherwise(expr),
        );
    }

    has_values.then_some(expr)
}

// Builds a typed boolean CASE expression for one column.
fn bool_case<'a, F>(
    packs: &'a UnitPacks<'a>,
    mut expr: UnitExpr<'a, Bool>,
    field: F,
) -> Option<UnitExpr<'a, Bool>>
where
    F: Fn(&UnitPack<'a>) -> Option<bool>,
{
    let mut has_values = false;

    for (id, pack) in packs {
        //
        let Some(value) = field(pack) else {
            continue;
        };

        has_values = true;

        expr = Box::new(
            diesel::dsl::case_when::<_, _, Bool>(f_id.eq(*id), value)
                .otherwise(expr),
        );
    }

    has_values.then_some(expr)
}

// Builds a typed double-precision CASE expression for one column.
fn double_case<'a, F>(
    packs: &'a UnitPacks<'a>,
    mut expr: UnitExpr<'a, Double>,
    field: F,
) -> Option<UnitExpr<'a, Double>>
where
    F: Fn(&UnitPack<'a>) -> Option<f64>,
{
    let mut has_values = false;

    for (id, pack) in packs {
        //
        let Some(value) = field(pack) else {
            continue;
        };

        has_values = true;

        expr = Box::new(
            diesel::dsl::case_when::<_, _, Double>(f_id.eq(*id), value)
                .otherwise(expr),
        );
    }

    has_values.then_some(expr)
}

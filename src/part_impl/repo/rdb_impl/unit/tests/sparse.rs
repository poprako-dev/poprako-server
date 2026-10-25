//! Real `PostgreSQL` coverage for sparse Unit persistence.

use std::sync::{Arc, Mutex};

use diesel::connection::InstrumentationEvent;
use diesel::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::{AsyncConnection as _, RunQueryDsl as _};
use time::OffsetDateTime;

use super::*;
use crate::part_impl::repo::rdb_impl::schema::t_unit;

// Construct an empty patch that may restore a hidden Unit.
fn patch(id: &str) -> UnitEdit {
    UnitEdit::Save {
        id: id.into(),
        next_id: Patch::Skip,
        is_bubble: None,
        is_flagged: None,
        coord: None,
        typography_note: Patch::Skip,
        translation_note: Patch::Skip,
        translation: Patch::Skip,
        revision: Patch::Skip,
    }
}

// Copy only Unit update statements recorded by the real connection.
fn updates(queries: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    queries
        .lock()
        .unwrap()
        .iter()
        .filter(|query| query.starts_with("UPDATE \"t_unit\""))
        .cloned()
        .collect()
}

// Verify omitted fields, structural no-ops, restoration, and combined inserts.
#[tokio::test]
#[serial_test::serial(repo_rdb)]
async fn sparse_unit_writes_use_testcontainer() {
    let database = crate::shared::test_rdb::start().await;

    let shared = database.core();

    let fixture = test_shared::seed_page(&shared, "sparse-unit-").await;

    let config = crate::config::repo::RepoConfig {
        unit_order_query_batch_size: std::num::NonZeroU32::MIN,
        ..Default::default()
    };

    let repo = RdbRepo::new(shared.clone(), config);

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let page_source_id = fixture.page_source_entry.id.as_str();

    let user_id = fixture.chapter_info.creator_id.as_str();

    let queries = Arc::new(Mutex::new(Vec::<String>::new()));

    nucl.coord(async |context| {
        let recorded = Arc::clone(&queries);

        context.conn().set_instrumentation(
            move |event: InstrumentationEvent<'_>| {
                if let InstrumentationEvent::StartQuery { query, .. } = event {
                    recorded.lock().unwrap().push(query.to_string());
                }
            },
        );

        let mut final_patch = patch("a");

        if let UnitEdit::Save {
            translation,
            is_flagged,
            ..
        } = &mut final_patch
        {
            *translation = Patch::Assign {
                value: UnitTranslation {
                    translated_text: "final".into(),
                    last_translator_id: user_id.into(),
                },
            };

            *is_flagged = Some(true);
        }

        let edits = [
            create_edit("a", user_id, "initial"),
            create_edit("b", user_id, "other"),
            final_patch,
        ];

        repo.step(
            context,
            &ApplyUnitEdits {
                page_source_id,
                orders: &[],
                edits: &edits,
            },
        )
        .await?;

        assert!(updates(&queries).is_empty());

        assert_eq!(queries.lock().unwrap().len(), 2);

        let units = repo
            .step(context, &ListUnitInfosByIds { ids: &["a"] })
            .await?;

        let unit = units.as_slice().first().unwrap();

        assert_eq!(unit.translated_text.as_deref(), Some("final"));

        assert_eq!(unit.created_at, unit.updated_at);

        assert!(unit.is_flagged);

        queries.lock().unwrap().clear();

        let orders = repo
            .step(context, &ListUnitOrders { page_source_id })
            .await?;

        // Two links require two queries with a configured batch size of one.
        assert_eq!(orders.len(), 2);

        assert_eq!(queries.lock().unwrap().len(), 2);

        diesel::update(
            t_unit::table.filter(t_unit::f_page_source_id.eq(page_source_id)),
        )
        .set(t_unit::f_updated_at.eq(OffsetDateTime::UNIX_EPOCH))
        .execute(context.conn())
        .await
        .map_err(crate::shared::result::diesel)?;

        queries.lock().unwrap().clear();

        let mut tail_patch = patch("b");

        if let UnitEdit::Save { next_id, .. } = &mut tail_patch {
            *next_id = Patch::Clear;
        }

        repo.step(
            context,
            &ApplyUnitEdits {
                page_source_id,
                orders: &orders,
                edits: &[patch("a"), tail_patch],
            },
        )
        .await?;

        assert!(updates(&queries).is_empty());

        assert_eq!(queries.lock().unwrap().len(), 2);

        let units = repo
            .step(context, &ListUnitInfosByIds { ids: &["a", "b"] })
            .await?;

        assert!(
            units
                .iter()
                .all(|unit| unit.updated_at == OffsetDateTime::UNIX_EPOCH)
        );

        let mut flag_patch = patch("a");

        if let UnitEdit::Save { is_flagged, .. } = &mut flag_patch {
            *is_flagged = Some(false);
        }

        queries.lock().unwrap().clear();

        repo.step(
            context,
            &ApplyUnitEdits {
                page_source_id,
                orders: &orders,
                edits: &[flag_patch],
            },
        )
        .await?;

        let statements = updates(&queries);

        assert_eq!(statements.len(), 1);

        let statement = statements.as_slice().first().unwrap();

        assert!(statement.contains("f_is_flagged"));

        assert!(statement.contains("f_updated_at"));

        assert_eq!(queries.lock().unwrap().len(), 2);

        assert_eq!(statement.matches('$').count(), 5);

        for column in [
            "f_next_id",
            "f_hidden_at",
            "f_is_bubble",
            "f_x_coord",
            "f_y_coord",
            "f_translated_text",
            "f_last_translator_id",
            "f_proofread_text",
            "f_last_proofreader_id",
            "f_is_proofread",
        ] {
            assert!(!statement.contains(column), "unexpected column: {column}");
        }

        let units = repo
            .step(context, &ListUnitInfosByIds { ids: &["a", "b"] })
            .await?;

        assert_eq!(
            units.iter().find(|unit| unit.id == "b").unwrap().updated_at,
            OffsetDateTime::UNIX_EPOCH
        );

        repo.step(
            context,
            &ApplyUnitEdits {
                page_source_id,
                orders: &orders,
                edits: &[UnitEdit::Delete { id: "a".into() }],
            },
        )
        .await?;

        let orders = repo
            .step(context, &ListUnitOrders { page_source_id })
            .await?;

        let before = repo
            .step(context, &ListUnitInfosByIds { ids: &["a"] })
            .await?;

        queries.lock().unwrap().clear();

        repo.step(
            context,
            &ApplyUnitEdits {
                page_source_id,
                orders: &orders,
                edits: &[UnitEdit::Delete { id: "a".into() }],
            },
        )
        .await?;

        assert!(updates(&queries).is_empty());

        let after = repo
            .step(context, &ListUnitInfosByIds { ids: &["a"] })
            .await?;

        assert_eq!(
            before.as_slice().first().unwrap().hidden_at,
            after.as_slice().first().unwrap().hidden_at
        );

        assert_eq!(
            before.as_slice().first().unwrap().updated_at,
            after.as_slice().first().unwrap().updated_at
        );

        queries.lock().unwrap().clear();

        repo.step(
            context,
            &ApplyUnitEdits {
                page_source_id,
                orders: &orders,
                edits: &[patch("a")],
            },
        )
        .await?;

        let statements = updates(&queries);

        assert_eq!(statements.len(), 1);

        assert!(
            statements
                .as_slice()
                .first()
                .unwrap()
                .contains("f_hidden_at")
        );

        assert!(!statements.as_slice().first().unwrap().contains("f_next_id"));

        let restored = repo
            .step(context, &ListUnitInfosByIds { ids: &["a"] })
            .await?;

        assert!(restored.as_slice().first().unwrap().hidden_at.is_none());

        assert_eq!(
            restored
                .as_slice()
                .first()
                .unwrap()
                .translated_text
                .as_deref(),
            Some("final")
        );

        accept!(())
    })
    .await
    .unwrap();
}

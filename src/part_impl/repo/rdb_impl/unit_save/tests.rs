// unit_save_receipts_are_atomic(UnitSaveRepo)(positive): concurrent replays commit one Unit and one receipt.
// unit_save_receipts_are_atomic(UnitSaveRepo)(negative): rollback leaves neither edits nor receipt.
use crate::data::instr::unit::{
    SavePageUnitEditsInstr, UnitCoordInstr, UnitEditInstr,
};
use crate::model::shared::user::UserToken;
use crate::model::write::assignment::AssignmentEntry;
use crate::part::nucl::Serial;
use crate::part::repo::oper::assignment::CreateAssignment;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::t_unit_save;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::accept;
use crate::shared::test_rdb::start;
use crate::usecase::unit::save_edits;
use crate::value::role::{RoleField, RoleMask};
use diesel::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, OperStep as _};

fn batch(page_id: &str, save_id: &str) -> SavePageUnitEditsInstr {
    SavePageUnitEditsInstr {
        page_id: page_id.into(),
        save_id: save_id.into(),
        edits: vec![UnitEditInstr::Create {
            local_id: "local".into(),
            next_id: None,
            is_bubble: true,
            is_flagged: false,
            coord: UnitCoordInstr {
                x_coord: 0.2,
                y_coord: 0.3,
            },
            translation: None,
            revision: None,
        }],
    }
}

#[tokio::test]
async fn unit_save_receipts_are_atomic() {
    let database = start().await;

    let shared = database.core();

    let fixture = test_shared::seed_page(&shared, "save-receipt-").await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<Serial>::new(shared.clone());

    let user_id = fixture.chapter_entry.creator_id.clone();

    let page_id = fixture.page_entry.id.clone();

    let entry = AssignmentEntry {
        id: "save-receipt-assignment".into(),
        chapter_id: fixture.chapter_entry.id.clone(),
        user_id: user_id.clone(),
        roles: RoleMask::from(RoleField::TRANSLATOR),
    };

    nucl.coord(async |context| {
        CreateAssignment { entry: &entry }
            .step_on(&repo, context)
            .await?;

        accept(())
    })
    .await
    .unwrap();

    let save_id = uuid::Uuid::new_v4().to_string();

    let (first, simultaneous) = tokio::join!(
        save_edits(
            (&nucl, &repo),
            UserToken {
                user_id: user_id.clone()
            },
            batch(&page_id, &save_id)
        ),
        save_edits(
            (&nucl, &repo),
            UserToken {
                user_id: user_id.clone()
            },
            batch(&page_id, &save_id)
        )
    );

    assert!(first.is_ok() || simultaneous.is_ok());

    let replay = save_edits(
        (&nucl, &repo),
        UserToken {
            user_id: user_id.clone(),
        },
        batch(&page_id, &save_id),
    )
    .await
    .unwrap();

    assert_eq!(replay.created_unit_ids.len(), 1);

    for successful in [first, simultaneous].into_iter().flatten() {
        assert_eq!(
            successful.created_unit_ids[0].unit_id,
            replay.created_unit_ids[0].unit_id
        );
    }

    let mut conn = shared.get().await.unwrap();

    let count = t_unit_save::table
        .filter(t_unit_save::f_page_id.eq(&page_id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(count, 1);

    let rejected_id = uuid::Uuid::new_v4().to_string();

    let mut rejected = batch(&page_id, &rejected_id);

    rejected.edits.push(UnitEditInstr::Delete {
        id: "missing".into(),
    });

    assert!(
        save_edits(
            (&nucl, &repo),
            UserToken {
                user_id: user_id.clone()
            },
            rejected
        )
        .await
        .is_err()
    );

    let count = t_unit_save::table
        .filter(t_unit_save::f_save_id.eq(&rejected_id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(count, 0);

    assert!(
        save_edits(
            (&nucl, &repo),
            UserToken { user_id },
            batch(&page_id, &rejected_id)
        )
        .await
        .is_ok()
    );
}

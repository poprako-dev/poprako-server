//! Prom-consumer worker that polls, dispatches by topic, and executes
//! deferred actions using coordinated repository operations.
//!
//! Topic dispatch routes to [`image`].

pub use base::RdbPromHandler;

/// Shared types and dispatch logic (extracted to avoid upward ancestor dependency).
mod base;
/// Prom chapter workflow handler.
mod chapter;
/// Prom image event handler.
mod image;
/// Prom invitation event handler.
mod invitation;
mod pool;
#[cfg(all(test, feature = "repo"))]
mod tests {
    // image_payloads_from_rdb_dispatch(dispatch_payload)(positive): payloads stored by the RDB defer path are decoded and dispatched by their topic.

    use super::*;

    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    use poprako_orchestra_extra::prom::task::Task;
    use time::OffsetDateTime;

    use crate::part::prom::payload::image;
    use crate::part_impl::drive::rdb_impl::RdbDrive;
    use crate::part_impl::prom::rdb_impl::entity::LocalMessageEntry;
    use crate::part_impl::prom::rdb_impl::handler::base::dispatch_payload;
    use crate::part_impl::prom::rdb_impl::handler::task_flow::TaskFlow;
    use crate::part_impl::repo::mock_impl::Mock;
    use crate::part_impl::repo::rdb_impl::{RdbRepo, schema, test_shared};

    const PREFIX: &str = "rdb-test-prom-handler-";

    #[tokio::test]
    async fn image_payloads_from_rdb_dispatch() {
        //
        let shared = test_shared::shared().await;

        test_shared::reset(&shared, PREFIX).await;

        let delete_id = "rdb-test-prom-handler-delete".to_string();

        let delete_payload = Payload::Image(image::Payload::Delete {
            object_key: "old-avatar.png".to_string(),
        });

        let delete_task = Task {
            id: &delete_id,
            payload: &delete_payload,
            delay: None,
        };

        let delete_local_message_entry = LocalMessageEntry::from_task(
            &delete_task,
            OffsetDateTime::now_utc(),
        )
        .ok()
        .unwrap();

        let mut conn = shared.get().await.ok().unwrap();

        diesel::insert_into(schema::t_local_message::table)
            .values(&delete_local_message_entry)
            .execute(&mut conn)
            .await
            .ok()
            .unwrap();

        let delete_payload: serde_json::Value = schema::t_local_message::table
            .filter(
                schema::t_local_message::f_id
                    .eq("rdb-test-prom-handler-delete"),
            )
            .select(schema::t_local_message::f_payload)
            .first(&mut conn)
            .await
            .ok()
            .unwrap();

        let nucl = RdbDrive::new(shared.clone());

        let rdb_prom_repo = RdbPromRepo::new(RdbRepo::new(shared.clone()));

        let image_pool = Mock::new();

        let delete_task_flow = dispatch_payload(
            &nucl,
            rdb_prom_repo.inner(),
            &image_pool,
            "image",
            &delete_payload,
        )
        .await;

        assert!(matches!(delete_task_flow, TaskFlow::Complete));

        assert_eq!(
            image_pool.snapshot().deleted_image_keys,
            vec!["old-avatar.png".to_string()]
        );

        let check_id = "rdb-test-prom-handler-check-uploaded".to_string();

        let check_payload = Payload::Image(image::Payload::CheckUpload {
            resource_kind: image::ResourceKind::UserAvatar,
            resource_id: "missing-user".to_string(),
            object_key: "new-avatar.png".to_string(),
            version: 1,
        });

        let check_task = Task {
            id: &check_id,
            payload: &check_payload,
            delay: None,
        };

        let check_uploaded_local_message_entry = LocalMessageEntry::from_task(
            &check_task,
            OffsetDateTime::now_utc(),
        )
        .ok()
        .unwrap();

        diesel::insert_into(schema::t_local_message::table)
            .values(&check_uploaded_local_message_entry)
            .execute(&mut conn)
            .await
            .ok()
            .unwrap();

        let check_uploaded_payload: serde_json::Value =
            schema::t_local_message::table
                .filter(
                    schema::t_local_message::f_id
                        .eq("rdb-test-prom-handler-check-uploaded"),
                )
                .select(schema::t_local_message::f_payload)
                .first(&mut conn)
                .await
                .ok()
                .unwrap();

        let image_pool = Mock::new().with_image_head_absent();

        let check_uploaded_task_flow = dispatch_payload(
            &nucl,
            rdb_prom_repo.inner(),
            &image_pool,
            "image",
            &check_uploaded_payload,
        )
        .await;

        assert!(matches!(check_uploaded_task_flow, TaskFlow::Complete));

        test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

        test_shared::assert_no_leftovers(&shared, PREFIX)
            .await
            .ok()
            .unwrap();
    }
}
mod task_flow;

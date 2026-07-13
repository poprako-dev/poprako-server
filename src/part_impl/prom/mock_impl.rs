//! Mock implementations of [`Prom`] for testing deferred action recording,
//! plus an on-demand prom-record processor for integration tests.

mod tests;

use poprako_orchestra::{Nucl as _, Step};
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;
use time::OffsetDateTime;

use crate::model::user::{UserCredential, UserInfo};
use crate::part::image::ImageManager;
use crate::part::prom::Prom;
use crate::part::prom::payload::{Payload, image};
use crate::part::repo::oper::comic::{
    GetComicInfoExcluded, MarkComicCoverUploaded,
};
use crate::part::repo::oper::page::{
    GetPageInfoExcluded, MarkPageImageUploaded,
};
use crate::part::repo::oper::team::{GetTeamInfoExcluded, UpdateTeam};
use crate::part::repo::oper::user::{GetUserInfoExcluded, UpdateUser};
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::{RegularError, RegularResult};

/// A recorded deferred action stored in the mock context during transactional testing.
#[cfg_attr(test, derive(Clone))]
pub struct MockPromRecord {
    id: String,

    /// Serialized JSON of the [`Payload`].
    ///
    /// Call [`payload`](MockPromRecord::payload) to deserialize on-the-fly
    /// for assertions.
    payload_json: String,

    visible_at: OffsetDateTime,
}

impl MockPromRecord {
    /// Returns the prom message id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the deferred visibility time.
    pub fn visible_at(&self) -> OffsetDateTime {
        self.visible_at
    }

    /// Deserializes the stored JSON back into a [`Payload`].
    ///
    pub fn payload(&self) -> Payload {
        serde_json::from_str(&self.payload_json)
            .expect("stored prom payload should deserialize successfully")
    }
}

/// Mock prom implementation used by coordinated tests.
impl Prom<MockContext> for Mock {}

/// Defers one record in the coordinated mock state.
impl<'a> Step<Defer<'a, String, Payload, ()>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &Defer<'a, String, Payload, ()>,
    ) -> Result<(), Self::Error> {
        //
        let payload_json =
            serde_json::to_string(oper.task.payload).map_err(|error| {
                RegularError::Unrecoverable {
                    message: format!(
                        "failed to serialize prom payload: {}",
                        error
                    ),
                }
            })?;

        context.state.prom_records.push(MockPromRecord {
            id: oper.task.id.to_string(),
            payload_json,
            visible_at: OffsetDateTime::now_utc()
                + oper.task.delay.unwrap_or_default(),
        });

        Ok(())
    }
}

impl<'t, 'a> Step<DeferBatch<'t, 'a, String, Payload, ()>, MockContext>
    for Mock
{
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeferBatch<'t, 'a, String, Payload, ()>,
    ) -> Result<(), Self::Error> {
        //
        for task in oper.tasks {
            self.step(
                context,
                &Defer::new(Task {
                    id: task.id,
                    payload: task.payload,
                    delay: task.delay,
                }),
            )
            .await?;
        }

        Ok(())
    }
}

async fn defer_payload(
    mock: &Mock,
    context: &mut MockContext,
    id: &str,
    payload: Payload,
) -> RegularResult<()> {
    //
    let id = id.to_string();

    let task = Task {
        id: &id,
        payload: &payload,
        delay: None,
    };

    mock.step(context, &Defer::new(task)).await
}

// defer_records_payload(Prom::step)(positive): individual deferral should store the record in transaction state.
// defer_batch_records_payloads(Prom::step)(positive): batch deferral should store every record in transaction state.
// process_pending_marks_uploaded_image(process_pending)(positive): check-upload should mark matching uploaded image state.
// process_pending_keeps_stale_image_for_ordered_delete(process_pending)(positive): stale check-upload should leave cleanup to ordered delete messages.
// process_pending_deletes_missing_resource_image(process_pending)(positive): check-upload should delete an object when the owning resource disappeared.

fn user_info(id: &str, avatar_key: &str, avatar_version: u32) -> UserInfo {
    //
    let now = OffsetDateTime::now_utc();

    UserInfo {
        id: id.to_string(),
        qid: format!("qid-{}", id),
        nickname: format!("nick-{}", id),
        avatar_key: Some(avatar_key.to_string()),
        avatar_uploaded: false,
        avatar_version,
        is_sadmin: false,
        last_active_at: now,
        created_at: now,
        updated_at: now,
    }
}

fn user_credential(id: &str) -> UserCredential {
    UserCredential {
        user_id: id.to_string(),
        password_hash: format!("hash-{}", id),
    }
}

#[tokio::test]
async fn defer_records_payload() {
    //
    let mock = Mock::new();

    let before = OffsetDateTime::now_utc();

    let prom = mock.clone();

    assert!(
        mock.coord(async move |context| {
            defer_payload(
                &prom,
                context,
                "prom-1",
                Payload::Image(image::Payload::Delete {
                    object_key: "key".to_string(),
                }),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .is_ok()
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.prom_records.len(), 1);

    assert_eq!(snapshot.prom_records[0].id(), "prom-1");

    assert!(snapshot.prom_records[0].visible_at() >= before);
}

#[tokio::test]
async fn process_pending_marks_uploaded_image() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_info("user-1", "avatar.png", 1),
        user_credential("user-1"),
    );

    let prom = mock.clone();

    mock.coord(async move |context| {
        //
        defer_payload(
            &prom,
            context,
            "prom-1",
            Payload::Image(image::Payload::CheckUpload {
                resource_kind: image::ResourceKind::UserAvatar,
                resource_id: "user-1".to_string(),
                object_key: "avatar.png".to_string(),
                version: 1,
            }),
        )
        .await?;

        Ok::<(), RegularError>(())
    })
    .await
    .ok()
    .unwrap();

    process_pending(&mock).await.ok().unwrap();

    assert!(mock.snapshot().users[0].avatar_uploaded);
}

#[tokio::test]
async fn process_pending_keeps_stale_image_for_ordered_delete() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_info("user-1", "avatar-v2.png", 2),
        user_credential("user-1"),
    );

    let prom = mock.clone();

    mock.coord(async move |context| {
        //
        defer_payload(
            &prom,
            context,
            "prom-1",
            Payload::Image(image::Payload::CheckUpload {
                resource_kind: image::ResourceKind::UserAvatar,
                resource_id: "user-1".to_string(),
                object_key: "avatar-v1.png".to_string(),
                version: 1,
            }),
        )
        .await?;

        Ok::<(), RegularError>(())
    })
    .await
    .ok()
    .unwrap();

    process_pending(&mock).await.ok().unwrap();

    let snapshot = mock.snapshot();

    assert!(!snapshot.users[0].avatar_uploaded);

    assert!(snapshot.deleted_image_keys.is_empty());
}

#[tokio::test]
async fn process_pending_deletes_missing_resource_image() {
    //
    let mock = Mock::new();

    let prom = mock.clone();

    mock.coord(async move |context| {
        //
        defer_payload(
            &prom,
            context,
            "prom-1",
            Payload::Image(image::Payload::CheckUpload {
                resource_kind: image::ResourceKind::UserAvatar,
                resource_id: "missing-user".to_string(),
                object_key: "orphan-avatar.png".to_string(),
                version: 1,
            }),
        )
        .await?;

        Ok::<(), RegularError>(())
    })
    .await
    .ok()
    .unwrap();

    process_pending(&mock).await.ok().unwrap();

    assert_eq!(
        mock.snapshot().deleted_image_keys,
        vec!["orphan-avatar.png".to_string()]
    );
}

// ── On-demand prom processor for integration tests ─────────────────────────

/// Process all pending prom records in mock state.
///
/// Deserializes each record's stored payload and
/// executes the same handler logic as the production handler against
/// [`Mock`]'s in-memory implementations of all ports.
///
/// Call this after a usecase has enqueued prom records to exercise
/// the full deferred-action chain within an integration test.
pub async fn process_pending(mock: &Mock) -> RegularResult<()> {
    //
    let snapshot = mock.snapshot();

    for record in &snapshot.prom_records {
        //
        let payload = record.payload();

        match payload {
            Payload::Image(task) => {
                process_image_task(mock, &task).await?;
            }
        }
    }

    Ok(())
}

/// Process a single image task against the mock's in-memory image pool.
async fn process_image_task(
    image_pool: &Mock,
    task: &image::Payload,
) -> RegularResult<()> {
    match task {
        //
        image::Payload::CheckUpload {
            resource_kind,
            resource_id,
            object_key,
            version,
        } => match image_pool.head_object(object_key).await? {
            //
            true => {
                process_existing_image(
                    image_pool,
                    *resource_kind,
                    resource_id,
                    object_key,
                    *version,
                )
                .await
            }

            false => Ok(()),
        },

        image::Payload::Delete { object_key } => {
            image_pool.delete_object(object_key).await
        }
    }
}

async fn process_existing_image(
    mock: &Mock,
    kind: image::ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
) -> RegularResult<()> {
    //
    let resource_state = mock
        .coord(async move |context| {
            match mark_uploaded(mock, context, kind, resource_id, image_version)
                .await
            {
                Ok(()) => Ok(ResourceState::Current),

                Err(RegularError::Expected { .. }) => {
                    classify_expected_mark(
                        mock,
                        context,
                        kind,
                        resource_id,
                        image_version,
                    )
                    .await
                }

                Err(error) => Err(error),
            }
        })
        .await?;

    match resource_state {
        //
        ResourceState::Current | ResourceState::Stale => Ok(()),

        ResourceState::Missing => mock.delete_object(object_key).await,
    }
}

async fn mark_uploaded(
    mock: &Mock,
    context: &mut MockContext,
    kind: image::ResourceKind,
    resource_id: &str,
    image_version: u32,
) -> RegularResult<()> {
    match kind {
        //
        image::ResourceKind::UserAvatar => {
            //

            mock.step(
                context,
                &UpdateUser::MarkAvatarUploaded {
                    id: resource_id,
                    avatar_version: image_version,
                },
            )
            .await
        }

        image::ResourceKind::TeamAvatar => {
            //

            mock.step(
                context,
                &UpdateTeam::MarkAvatarUploaded {
                    id: resource_id,
                    avatar_version: image_version,
                },
            )
            .await
        }

        image::ResourceKind::ComicCover => {
            mock.step(
                context,
                &MarkComicCoverUploaded {
                    id: resource_id,
                    cover_version: image_version,
                },
            )
            .await
        }

        image::ResourceKind::PageImage => {
            mock.step(
                context,
                &MarkPageImageUploaded {
                    id: resource_id,
                    image_version,
                },
            )
            .await
        }
    }
}

enum ResourceState {
    Current,
    Stale,
    Missing,
}

fn classify_current_version(
    current_version: u32,
    image_version: u32,
    error_message: &'static str,
) -> RegularResult<ResourceState> {
    match current_version == image_version {
        //
        true => Err(RegularError::Unrecoverable {
            message: error_message.into(),
        }),

        false => Ok(ResourceState::Stale),
    }
}

async fn classify_expected_mark(
    mock: &Mock,
    context: &mut MockContext,
    kind: image::ResourceKind,
    resource_id: &str,
    image_version: u32,
) -> RegularResult<ResourceState> {
    match kind {
        //
        image::ResourceKind::UserAvatar => {
            //

            match mock
                .step(context, &GetUserInfoExcluded::Id { id: resource_id })
                .await
            {
                //
                Ok(user_info) => classify_current_version(
                    user_info.avatar_version,
                    image_version,
                    "[MockProm::classify_expected_mark] current user avatar version failed to mark uploaded",
                ),

                Err(RegularError::Expected { .. }) => {
                    Ok(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        image::ResourceKind::TeamAvatar => {
            //

            match mock
                .step(context, &GetTeamInfoExcluded::Id { id: resource_id })
                .await
            {
                //
                Ok(team_info) => classify_current_version(
                    team_info.avatar_version,
                    image_version,
                    "[MockProm::classify_expected_mark] current team avatar version failed to mark uploaded",
                ),

                Err(RegularError::Expected { .. }) => {
                    Ok(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        image::ResourceKind::ComicCover => {
            match mock
                .step(
                    context,
                    &GetComicInfoExcluded {
                        id: resource_id,
                        incls: &[],
                    },
                )
                .await
            {
                Ok(comic_info) => classify_current_version(
                    comic_info.cover_version,
                    image_version,
                    "[MockProm::classify_expected_mark] current comic cover version failed to mark uploaded",
                ),

                Err(RegularError::Expected { .. }) => {
                    Ok(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }

        image::ResourceKind::PageImage => {
            match mock
                .step(context, &GetPageInfoExcluded { id: resource_id })
                .await
            {
                Ok(page_info) => classify_current_version(
                    page_info.image_version,
                    image_version,
                    "[MockProm::classify_expected_mark] current page image version failed to mark uploaded",
                ),

                Err(RegularError::Expected { .. }) => {
                    Ok(ResourceState::Missing)
                }

                Err(error) => Err(error),
            }
        }
    }
}

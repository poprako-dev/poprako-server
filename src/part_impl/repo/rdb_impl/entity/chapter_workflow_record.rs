//! Diesel entity types for immutable chapter workflow records.

use diesel::prelude::*;
use serde_json::Value;
use time::OffsetDateTime;

use crate::model::read::proj::chapter_workflow_record::ChapterWorkflowRecordInfo;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part_impl::repo::rdb_impl::schema::t_chapter_workflow_record;
use crate::result::BaseError;
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordKind, ChapterWorkflowRecordPayload,
};

/// Raw database row returned from `t_chapter_workflow_record`.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_chapter_workflow_record)]
pub struct ChapterWorkflowRecordInfoRow {
    pub f_id: String,
    pub f_chapter_id: String,
    pub f_actor_user_id: Option<String>,
    pub f_kind: String,
    pub f_payload: Value,
    pub f_created_at: OffsetDateTime,
}

impl TryFrom<ChapterWorkflowRecordInfoRow> for ChapterWorkflowRecordInfo {
    type Error = BaseError;

    fn try_from(
        row: ChapterWorkflowRecordInfoRow,
    ) -> Result<Self, Self::Error> {
        //
        let kind = serde_json::from_value::<ChapterWorkflowRecordKind>(
            Value::String(row.f_kind.clone()),
        )
        .map_err(|error| {
            //
            tracing::error!(
                operation = "decode_chapter_workflow_record_kind",
                record_id = %row.f_id,
                err = ?error,
                "persisted chapter workflow record is corrupt",
            );

            BaseError::Unrecoverable {
                message: "persisted chapter workflow record kind is corrupt"
                    .into(),
            }
        })?;

        let payload = ChapterWorkflowRecordPayload::from_storage_json(
            kind,
            row.f_payload,
        )
        .map_err(|error| {
            //
            tracing::error!(
                operation = "decode_chapter_workflow_record_payload",
                record_id = %row.f_id,
                kind = ?kind,
                err = ?error,
                "persisted chapter workflow record is corrupt",
            );

            BaseError::Unrecoverable {
                message: "persisted chapter workflow record payload is corrupt"
                    .into(),
            }
        })?;

        Ok(Self {
            id: row.f_id,
            chapter_id: row.f_chapter_id,
            actor_user_id: row.f_actor_user_id,
            kind,
            payload,
            created_at: row.f_created_at,
        })
    }
}

/// Insertable immutable workflow record row.
#[derive(Insertable)]
#[diesel(table_name = t_chapter_workflow_record)]
pub struct ChapterWorkflowRecordEntryRow<'a> {
    pub f_id: &'a str,
    pub f_chapter_id: &'a str,
    pub f_actor_user_id: Option<&'a str>,
    pub f_kind: String,
    pub f_payload: Value,
    pub f_created_at: OffsetDateTime,
}

impl<'a> From<&'a ChapterWorkflowRecordEntry>
    for ChapterWorkflowRecordEntryRow<'a>
{
    fn from(entry: &'a ChapterWorkflowRecordEntry) -> Self {
        //
        Self {
            f_id: &entry.id,
            f_chapter_id: &entry.chapter_id,
            f_actor_user_id: entry.actor_user_id.as_deref(),
            f_kind: entry.payload.kind().as_str().to_string(),
            f_payload: entry.payload.to_storage_json(),
            f_created_at: entry.created_at,
        }
    }
}

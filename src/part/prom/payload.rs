use serde::{Deserialize, Serialize};

use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::image::ImagePayload;
use crate::part::prom::payload::invitation::InvitationPayload;

/// Deferred chapter payloads.
pub mod chapter;
/// Deferred image payloads.
pub mod image;
/// Deferred invitation payloads.
pub mod invitation;

/// One deferred task, grouped by its domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPayload {
    /// Advance raw provision after every chapter page is uploaded.
    #[serde(rename = "AdvanceRawProvide")]
    Chapter(ChapterPayload),

    /// Image-domain deferred action.
    Image(ImagePayload),

    /// Purge an invitation when it is still pending at its expiry time.
    #[serde(rename = "PurgeExpiredInvitation")]
    Invitation(InvitationPayload),
}

impl TaskPayload {
    /// Returns the routing topic string (e.g. `"image"`) for this payload.
    pub fn topic(&self) -> &'static str {
        match self {
            //
            Self::Chapter(_) => "advance_raw_provide",

            Self::Image(_) => "image",

            Self::Invitation(_) => "purge_expired_invitation",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // task_payload_serde(persisted_json)(positive): renamed task types preserve existing queue records.
    #[test]
    fn preserves_persisted_tags() {
        let chapter_task = TaskPayload::Chapter(ChapterPayload::RawProvide {
            chapter_id: "chapter-1".to_string(),
        });

        let chapter_json = serde_json::to_value(&chapter_task).unwrap();

        assert_eq!(
            chapter_json,
            serde_json::json!({ "AdvanceRawProvide": { "chapter_id": "chapter-1" } }),
        );

        let invitation_task = TaskPayload::Invitation(InvitationPayload::Member {
            invitation_id: "invitation-1".to_string(),
        });

        let invitation_json = serde_json::to_value(&invitation_task).unwrap();

        assert_eq!(
            invitation_json,
            serde_json::json!({
                "PurgeExpiredInvitation": { "Member": { "invitation_id": "invitation-1" } },
            }),
        );

        let decoded_task: TaskPayload =
            serde_json::from_value(invitation_json).unwrap();

        assert_eq!(decoded_task, invitation_task);
    }
}

//! Instr DTOs for the announcement domain.

//! Data transfer objects for announcement use cases.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::model::read::spec::announcement::AnnouncementListSpec;
use crate::value::announcement::AnnouncementInclOpt;

/// Input parameters for listing announcements.
///
/// `incl` embeds related rows into each item.
///
/// Example: `/api/v1/announcements?team_id=t_1&incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListAnnouncementInfosInstr {
    //
    /// Parent team whose announcements to list.
    pub team_id: String,

    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<AnnouncementInclOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

impl From<ListAnnouncementInfosInstr> for AnnouncementListSpec {
    // Map listing parameters directly to the repository spec.
    fn from(instr: ListAnnouncementInfosInstr) -> Self {
        Self {
            team_id: instr.team_id,
            incl_opt: instr.incl_opt,
            offset: instr.offset,
            limit: instr.limit,
        }
    }
}

/// Input parameters for creating an announcement.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAnnouncementInstr {
    //
    /// Target team identifier.
    pub team_id: String,

    /// Announcement title.
    pub title: String,
    /// Announcement body content.
    pub content: String,
}

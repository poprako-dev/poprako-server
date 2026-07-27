use poprako_orchestra::Oper;

use crate::model::announcement::{
    AnnouncementEntry, AnnouncementInfo, AnnouncementListSpec,
};

/// Lists announcement infos selected by a query specification.
pub struct ListAnnouncementInfos<'a> {
    /// Query specification for filtering announcements.
    pub spec: &'a AnnouncementListSpec,
}

impl Oper for ListAnnouncementInfos<'_> {
    // Internal output type for this step.
    type Output = Vec<AnnouncementInfo>;
}

/// Creates an announcement.
pub struct CreateAnnouncement<'a> {
    /// The announcement entry data.
    pub entry: &'a AnnouncementEntry,
}

impl Oper for CreateAnnouncement<'_> {
    // Internal output type for this step.
    type Output = AnnouncementInfo;
}

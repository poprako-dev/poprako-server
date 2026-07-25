use poprako_orchestra::Oper;

use crate::model::announcement::{AnnouncementEntry, AnnouncementInfo, AnnouncementListSpec};

/// Lists announcement infos selected by a query specification.
pub struct ListAnnouncementInfos<'a> {
    pub spec: &'a AnnouncementListSpec,
}

impl Oper for ListAnnouncementInfos<'_> {
    type Output = Vec<AnnouncementInfo>;
}

/// Creates an announcement.
pub struct CreateAnnouncement<'a> {
    pub entry: &'a AnnouncementEntry,
}

impl Oper for CreateAnnouncement<'_> {
    type Output = AnnouncementInfo;
}

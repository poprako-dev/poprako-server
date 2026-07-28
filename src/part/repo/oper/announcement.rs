use poprako_orchestra::Oper;

use crate::model::announcement::{
    AnnouncementEntry, AnnouncementInfo, AnnouncementListSpec,
};

/// Lists announcement infos selected by a query specification.
#[derive(Oper)]
#[oper(output = Vec<AnnouncementInfo>)]
pub struct ListAnnouncementInfos<'a> {
    /// Query specification for filtering announcements.
    pub spec: &'a AnnouncementListSpec,
}

/// Creates an announcement.
#[derive(Oper)]
#[oper(output = AnnouncementInfo)]
pub struct CreateAnnouncement<'a> {
    /// The announcement entry data.
    pub entry: &'a AnnouncementEntry,
}

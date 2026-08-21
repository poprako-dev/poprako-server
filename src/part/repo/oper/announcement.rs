use poprako_orchestra::Oper;

use crate::model::read::proj::announcement::AnnouncementInfo;
use crate::model::read::spec::announcement::AnnouncementListSpec;
use crate::model::write::announcement::{AnnouncementEntry, AnnouncementRepl};

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

/// Looks up and locks an announcement by identifier for mutation.
#[derive(Oper)]
#[oper(output = AnnouncementInfo)]
pub struct GetAnnouncementInfoExcluded<'a> {
    /// The announcement identifier.
    pub id: &'a str,
}

/// Replaces an announcement's editable fields.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdateAnnouncement<'a> {
    /// The replacement announcement content.
    pub update: &'a AnnouncementRepl,
}

/// Deletes an announcement by identifier.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteAnnouncement<'a> {
    /// The announcement identifier.
    pub id: &'a str,
}

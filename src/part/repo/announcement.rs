//! Repository traits for the announcement domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::announcement::{
    CreateAnnouncement, DeleteAnnouncement, GetAnnouncementInfo,
    ListAnnouncementInfos, UpdateAnnouncement,
};
use crate::result::BaseError;

/// Announcement repository operations.
///
/// Announcement operations execute independently through [`poprako_orchestra::Run`].
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> ListAnnouncementInfos<'a>,
        for<'a> CreateAnnouncement<'a>,
        for<'a> GetAnnouncementInfo<'a>,
        for<'a> UpdateAnnouncement<'a>,
        for<'a> DeleteAnnouncement<'a>,
    ),
)]
pub trait AnnouncementRepo<C> {}

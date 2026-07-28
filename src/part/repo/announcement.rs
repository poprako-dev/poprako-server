//! Repository traits for the announcement domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::announcement::{
    CreateAnnouncement, ListAnnouncementInfos,
};
use crate::result::BaseError;

/// Announcement repository operations.
///
/// Independent lists use [`poprako_orchestra::Run`], while creation steps through the context
/// coordinated by the caller.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> ListAnnouncementInfos<'a>,
    ),
    step(
        for<'a> CreateAnnouncement<'a>,
    ),
)]
pub trait AnnouncementRepo<C> {}

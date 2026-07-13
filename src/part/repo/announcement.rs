//! Repository traits for the announcement domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::announcement::{
    CreateAnnouncement, ListAnnouncementInfos,
};
use crate::result::RegularError;

/// Announcement repository operations.
///
/// Independent lists use [`Run`], while creation steps through the context
/// coordinated by the caller.
pub trait AnnouncementRepo<C>:
    for<'a> Run<ListAnnouncementInfos<'a>, Error = RegularError>
    + for<'a> Step<CreateAnnouncement<'a>, C, Error = RegularError>
{
}

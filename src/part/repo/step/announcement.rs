//! Step types for announcement repository opers.

use poprako_transactional::step::Step;

use crate::model::announcement::{
    AnnouncementForm, AnnouncementInfo, AnnouncementListSpec,
};

/// Step that lists announcements by query specification.
pub struct ListInfos<'a> {
    pub spec: &'a AnnouncementListSpec,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<AnnouncementInfo>;
}

/// Step that inserts a new announcement row.
pub struct Create<'a> {
    pub form: &'a AnnouncementForm,
}

impl<'a> Step for Create<'a> {
    type Output = AnnouncementInfo;
}

/// Factory for constructing announcement repository [`Step`] values.
pub struct AnnouncementStep;

impl AnnouncementStep {
    /// Constructs a step to list announcements.
    pub fn list_infos<'a>(spec: &'a AnnouncementListSpec) -> ListInfos<'a> {
        ListInfos { spec }
    }

    /// Constructs a step to insert a new announcement.
    pub fn create<'a>(form: &'a AnnouncementForm) -> Create<'a> {
        Create { form }
    }
}

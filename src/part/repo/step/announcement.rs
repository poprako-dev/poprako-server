//! Step types for announcement repository opers.

use poprako_transactional::step::Step;

use crate::model::announcement_model;

/// Step that lists announcements by query specification.
pub struct ListInfos<'a> {
    pub spec: &'a announcement_model::ListSpec,
}

impl<'a> Step for ListInfos<'a> {
    type Output = Vec<announcement_model::Info>;
}

/// Step that inserts a new announcement row.
pub struct Create<'a> {
    pub form: &'a announcement_model::Form,
}

impl<'a> Step for Create<'a> {
    type Output = announcement_model::Info;
}

/// Factory for constructing announcement repository [`Step`] values.
pub struct AnnouncementStep;

impl AnnouncementStep {
    /// Constructs a step to list announcements.
    pub fn list_infos<'a>(
        spec: &'a announcement_model::ListSpec,
    ) -> ListInfos<'a> {
        ListInfos { spec }
    }

    /// Constructs a step to insert a new announcement.
    pub fn create<'a>(form: &'a announcement_model::Form) -> Create<'a> {
        Create { form }
    }
}

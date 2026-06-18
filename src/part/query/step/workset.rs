use poprako_transactional::step::Step;

use crate::model::workset::WorksetInfo;

pub struct ListByTeamIdExcluded<'a> {
    pub team_id: &'a str,
}

impl<'a> Step for ListByTeamIdExcluded<'a> {
    type Output = Vec<WorksetInfo>;
}

pub struct DeleteCascade<'a> {
    pub id: &'a str,
}

impl<'a> Step for DeleteCascade<'a> {
    type Output = ();
}

pub struct WorksetStep;

impl WorksetStep {
    pub fn list_by_team_id_excluded<'a>(team_id: &'a str) -> ListByTeamIdExcluded<'a> {
        ListByTeamIdExcluded { team_id }
    }

    pub fn delete_cascade<'a>(id: &'a str) -> DeleteCascade<'a> {
        DeleteCascade { id }
    }
}

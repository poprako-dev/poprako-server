use poprako_transactional::step::Step;

use crate::model::workset::WorksetInfo;

pub struct WorksetListByTeamIdExcluded<'a> {
    pub team_id: &'a str,
}

impl<'a> Step for WorksetListByTeamIdExcluded<'a> {
    type Output = Vec<WorksetInfo>;
}

pub struct WorksetDeleteCascade<'a> {
    pub id: &'a str,
}

impl<'a> Step for WorksetDeleteCascade<'a> {
    type Output = ();
}

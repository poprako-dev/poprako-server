use time::Duration;

use crate::domain::complex::workset::WorksetComplex;
use crate::domain::local_message::message::ImageLocalMessage;
use crate::domain::query::QueryTransactional;
use crate::domain::query::local_message::LocalMessageQueryTransactional;
use crate::domain::query::team::TeamQueryTransactional;
use crate::domain::query::workset::WorksetQueryTransactional;
use crate::domain::result::DomainResult;

pub struct TeamComplex;

impl TeamComplex {
    /// Deletes the team, cascade-deletes all its worksets via
    /// [`super::workset::delete_cascade`], and queues a local message to delete
    /// the team avatar if one was present.
    pub async fn delete_cascade<Q>(query: &mut Q, id: &str) -> DomainResult<()>
    where
        Q: QueryTransactional,
    {
        // Read avatar key before deletion so we can schedule cleanup.
        let team = TeamQueryTransactional::get_by_id_excluded(query, id).await?;
        let avatar_key = team.avatar_key;

        // Cascade-delete each workset through its own delete_cascade.
        let worksets = WorksetQueryTransactional::list_by_team_id_excluded(query, id).await?;
        for workset in &worksets {
            WorksetComplex::delete_cascade(query, &workset.id).await?;
        }

        TeamQueryTransactional::delete(query, id).await?;

        // Queue avatar file deletion if there was one.
        if let Some(object_key) = avatar_key {
            let message = ImageLocalMessage::delete(object_key).into_form(Duration::seconds(0));
            LocalMessageQueryTransactional::append(query, &message).await?;
        }

        Ok(())
    }
}

pub struct TeamPermissionComplex;

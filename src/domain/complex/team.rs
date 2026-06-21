// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use time::Duration;
// 
// use crate::domain::complex::workset::WorksetComplex;
// use crate::domain::model::aggr::local_message::LocalMessageForm;
// use crate::domain::model::value::local_message::ImageLocalMessage;
// use crate::domain::repo_legacy::RepoTransactional;
// use crate::domain::repo_legacy::local_message::LocalMessageRepoTransactional;
// use crate::domain::repo_legacy::team::TeamRepoTransactional;
// use crate::domain::repo_legacy::workset::WorksetRepoTransactional;
// use crate::domain::result::DomainResult;
// 
// pub struct TeamComplex;
// 
// impl TeamComplex {
//     /// Deletes the team, cascade-deletes all its worksets via
//     /// [`super::workset::delete_cascade`], and queues a local message to delete
//     /// the team avatar if one was present.
//     pub async fn delete_cascade<R>(repo: &mut R, id: &str) -> DomainResult<()>
//     where
//         R: RepoTransactional,
//     {
//         // Read avatar key before deletion so we can schedule cleanup.
//         let team = TeamRepoTransactional::get_by_id_excluded(repo, id).await?;
//         let avatar_key = team.avatar_key;
// 
//         // Cascade-delete each workset through its own delete_cascade.
//         let worksets = WorksetRepoTransactional::list_by_team_id_excluded(repo, id).await?;
//         for workset in &worksets {
//             WorksetComplex::delete_cascade(repo, &workset.id).await?;
//         }
// 
//         TeamRepoTransactional::delete(repo, id).await?;
// 
//         // Queue avatar file deletion if there was one.
//         if let Some(object_key) = avatar_key {
//             let message = LocalMessageForm::from_image_message(
//                 ImageLocalMessage::delete(object_key),
//                 Duration::seconds(0),
//             );
//             LocalMessageRepoTransactional::append(repo, &message).await?;
//         }
// 
//         Ok(())
//     }
// }
// 
// pub struct TeamPermissionComplex;

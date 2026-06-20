// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use serde::{Deserialize, Serialize};
// use utoipa::ToSchema;
// 
// use poprako_util::time::ToUnixMilli as _;
// 
// use crate::domain::external::image_pool::ImageGet;
// use crate::domain::model::aggr::team::TeamAggr;
// 
// /// Public-facing representation of a translation team.
// #[derive(Debug, Serialize, ToSchema)]
// pub struct TeamInfo {
//     pub id: String,
// 
//     pub name: String,
//     pub description: String,
// 
//     pub avatar_url: Option<String>,
// 
//     pub workset_next_index: i32,
// 
//     pub created_at: i64,
//     pub updated_at: i64,
// }
// 
// impl TeamInfo {
//     pub async fn from_aggr<S>(aggr: TeamAggr, signer: &S) -> Self
//     where
//         S: ImageGet,
//     {
//         let avatar_url = match (aggr.avatar_uploaded, &aggr.avatar_key) {
//             (true, Some(key)) => signer.get_signed(key).await.ok(),
//             _ => None,
//         };
// 
//         Self {
//             id: aggr.id,
//             name: aggr.name,
//             description: aggr.description,
//             avatar_url: avatar_url.map(Into::into),
//             workset_next_index: aggr.workset_next_index,
//             created_at: aggr.created_at.to_unix_milli(),
//             updated_at: aggr.updated_at.to_unix_milli(),
//         }
//     }
// }
// 
// #[derive(Debug, Deserialize, ToSchema)]
// pub struct CreateParams {
//     pub name: String,
//     pub description: String,
// }
// 
// #[derive(Debug, Deserialize, ToSchema)]
// pub struct InfoUpdateParams {
//     pub name: String,
//     pub description: String,
// }
// 
// #[derive(Debug, Deserialize, ToSchema)]
// pub struct AvatarReserveParams {
//     pub file_extension: String,
// }
// 
// #[derive(Debug, Serialize, ToSchema)]
// pub struct AvatarReserveReply {
//     pub put_url: String,
//     pub avatar_version: i64,
// }
// 
// #[derive(Debug, Deserialize, ToSchema)]
// pub struct AvatarMarkUploadedParams {
//     pub avatar_version: i64,
// }

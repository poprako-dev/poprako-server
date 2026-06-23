// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use time::Duration;
// 
// use crate::domain::external::token::{TokenParse, TokenSign};
// use crate::domain::model::aggr::local_message::LocalMessageForm;
// use crate::domain::model::aggr::user::UserToken;
// use crate::domain::model::value::local_message::ImageLocalMessage;
// use crate::domain::repo_legacy::RepoTransactional;
// use crate::domain::repo_legacy::local_message::LocalMessageRepoTransactional;
// use crate::domain::repo_legacy::member::MemberRepoTransactional;
// use crate::domain::repo_legacy::user::UserRepoTransactional;
// use crate::domain::result::{DomainError, DomainResult};
// 
// pub struct UserComplex;
// 
// impl UserComplex {
//     /// Hashes a password with bcrypt using the default cost factor.
//     pub fn hash_password(password: &str) -> DomainResult<String> {
//         Self::hash_password_at_cost(password, bcrypt::DEFAULT_COST)
//     }
// 
//     /// Same as [`hash_password`] but with an explicit cost factor (for testing).
//     fn hash_password_at_cost(password: &str, cost: u32) -> DomainResult<String> {
//         bcrypt::hash(password, cost).map_err(|e| {
//             DomainError::unrecoverable(format!(
//                 "[user::hash_password] bcrypt hashing failed: {}",
//                 e
//             ))
//         })
//     }
// 
//     pub fn sign_token<S>(signer: &S, unsigned_token: &UserToken) -> DomainResult<String>
//     where
//         S: TokenSign,
//     {
//         signer.sign(unsigned_token)
//     }
// 
//     pub fn parse_token<P>(parser: &P, signed_token: &str) -> DomainResult<UserToken>
//     where
//         P: TokenParse,
//     {
//         parser.parse(signed_token)
//     }
// 
//     /// Deletes the user and all their member records across teams, and queues
//     /// a local message to delete the user avatar if one was present.
//     pub async fn delete_cascade<R>(repo: &mut R, id: &str) -> DomainResult<()>
//     where
//         R: RepoTransactional,
//     {
//         // Read avatar key before deletion so we can schedule cleanup.
//         let user = UserRepoTransactional::get_by_id_excluded(repo, id).await?;
// 
//         let avatar_key = user.avatar_key;
// 
//         // Delete each member record belonging to this user.
//         let members = MemberRepoTransactional::list_by_user_id_excluded(repo, id).await?;
//         for m in &members {
//             MemberRepoTransactional::delete(repo, &m.id).await?;
//         }
// 
//         UserRepoTransactional::delete(repo, id).await?;
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
// pub struct UserPermissionComplex;
// 
// #[cfg(test)]
// mod tests {
//     // hash_password_returns_bcrypt_prefix(hash_password)(positive): hashing should return a bcrypt hash with the expected prefix.
//     // hash_password_same_input_produces_different_hashes(hash_password)(positive): hashing the same password twice should use different salts.
//     // hash_password_empty_string(hash_password)(positive): hashing an empty password should still return a bcrypt hash.
//     // hash_password_can_be_verified_by_bcrypt(hash_password)(positive): bcrypt should verify the original password and reject a wrong one.
//     // hash_password_returns_error_on_invalid_cost(hash_password_at_cost)(negative): passing an out-of-range cost should return an Unrecoverable error.
//     // sign_token_delegates_to_codec(sign_token)(positive): token signing should delegate to the provided codec.
//     // sign_token_returns_codec_error(sign_token)(negative): token signing should propagate codec errors.
//     // parse_token_delegates_to_codec(parse_token)(positive): token parsing should delegate to the provided codec.
//     // parse_token_returns_codec_error(parse_token)(negative): token parsing should propagate codec errors.
// 
//     use super::*;
// 
//     use crate::domain::external::token::{TokenParse, TokenSign};
//     use crate::domain::model::aggr::user::UserToken;
//     use crate::domain::result::{DomainError, DomainResult};
// 
//     struct FakeCodec {
//         fail: bool,
//     }
// 
//     impl TokenSign for FakeCodec {
//         fn sign(&self, unsigned_token: &UserToken) -> DomainResult<String> {
//             if self.fail {
//                 return Err(DomainError::unrecoverable(
//                     "[FakeCodec::sign] sign failed".into(),
//                 ));
//             }
// 
//             Ok(format!("signed:{}", unsigned_token.user_id))
//         }
//     }
// 
//     impl TokenParse for FakeCodec {
//         fn parse(&self, signed_token: &str) -> DomainResult<UserToken> {
//             if self.fail {
//                 return Err(DomainError::unrecoverable(
//                     "[FakeCodec::parse] parse failed".into(),
//                 ));
//             }
// 
//             Ok(UserToken {
//                 user_id: signed_token.replace("signed:", ""),
//             })
//         }
//     }
// 
//     #[test]
//     fn hash_password_returns_bcrypt_prefix() {
//         let hash = UserComplex::hash_password("my-password").unwrap();
//         assert!(hash.starts_with("$2b$"));
//     }
// 
//     #[test]
//     fn hash_password_same_input_produces_different_hashes() {
//         let h1 = UserComplex::hash_password("my-password").unwrap();
//         let h2 = UserComplex::hash_password("my-password").unwrap();
//         assert_ne!(h1, h2, "bcrypt must use a random salt for each hash");
//     }
// 
//     #[test]
//     fn hash_password_empty_string() {
//         let hash = UserComplex::hash_password("").unwrap();
//         assert!(hash.starts_with("$2b$"));
//     }
// 
//     #[test]
//     fn hash_password_can_be_verified_by_bcrypt() {
//         let hash = UserComplex::hash_password("my-password").unwrap();
//         assert!(bcrypt::verify("my-password", &hash).unwrap());
//         assert!(!bcrypt::verify("wrong-password", &hash).unwrap());
//     }
// 
//     #[test]
//     fn hash_password_returns_error_on_invalid_cost() {
//         let err = UserComplex::hash_password_at_cost("password", 100)
//             .err()
//             .unwrap();
//         assert!(matches!(err, DomainError::Unrecoverable { .. }));
//     }
// 
//     #[test]
//     fn sign_token_delegates_to_codec() {
//         let codec = FakeCodec { fail: false };
//         let token = UserToken {
//             user_id: "user-1".into(),
//         };
// 
//         let signed = UserComplex::sign_token(&codec, &token).unwrap();
// 
//         assert_eq!(signed, "signed:user-1");
//     }
// 
//     #[test]
//     fn sign_token_returns_codec_error() {
//         let codec = FakeCodec { fail: true };
//         let token = UserToken {
//             user_id: "user-1".into(),
//         };
// 
//         let err = UserComplex::sign_token(&codec, &token).err().unwrap();
// 
//         assert!(matches!(err, DomainError::Unrecoverable { .. }));
//     }
// 
//     #[test]
//     fn parse_token_delegates_to_codec() {
//         let codec = FakeCodec { fail: false };
// 
//         let parsed = UserComplex::parse_token(&codec, "signed:user-1").unwrap();
// 
//         assert_eq!(parsed.user_id, "user-1");
//     }
// 
//     #[test]
//     fn parse_token_returns_codec_error() {
//         let codec = FakeCodec { fail: true };
// 
//         let err = UserComplex::parse_token(&codec, "signed:user-1")
//             .err()
//             .unwrap();
// 
//         assert!(matches!(err, DomainError::Unrecoverable { .. }));
//     }
// }

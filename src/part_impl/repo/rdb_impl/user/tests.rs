// user_roundtrip_uses_testcontainer(GetUserInfo, GetUserCredential, FindUserInfo)(positive): user repo persists and reloads a user from an isolated PostgreSQL container.

use super::*;

use crate::part::repo::oper::user::{
    FindUserInfo, GetUserCredential, GetUserInfo,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::part_impl::shared::RdbCore;

const PREFIX: &str = "rdb-test-user-domain-";

pub async fn user_roundtrip_uses_testcontainer(shared: RdbCore) {
    test_shared::reset(&shared, PREFIX).await;

    let user_fixture = test_shared::seed_user(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let user_info = repo
        .run(&GetUserInfo::Id {
            id: &user_fixture.user_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(user_info.id, user_fixture.user_entry.id);

    let user_credential = repo
        .run(&GetUserCredential::Qid {
            qid: &user_fixture.user_entry.qid,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(user_credential.user_id, user_fixture.user_entry.id);

    let found_user_info = repo
        .run(&FindUserInfo::Qid {
            qid: &user_fixture.user_entry.qid,
        })
        .await
        .ok()
        .unwrap()
        .unwrap();

    assert_eq!(found_user_info.id, user_fixture.user_entry.id);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}

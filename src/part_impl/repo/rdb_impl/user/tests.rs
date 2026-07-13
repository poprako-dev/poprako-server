// user_roundtrip_reads_test_database_url(GetUserInfo, GetUserCredential, FindUserInfo)(positive): user repo persists and reloads a user from the local test database.

use super::*;

use crate::part::repo::oper::user::{
    FindUserInfo, GetUserCredential, GetUserInfo,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};

const PREFIX: &str = "rdb-test-user-domain-";

#[tokio::test]
async fn user_roundtrip_reads_test_database_url() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let user_fixture = test_shared::seed_user(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let get_user_info = GetUserInfo::Id {
        id: &user_fixture.user_entry.id,
    };

    let user_info = repo.run(&get_user_info).await.ok().unwrap();

    assert_eq!(user_info.id, user_fixture.user_entry.id);

    let get_user_credential = GetUserCredential::Qid {
        qid: &user_fixture.user_entry.qid,
    };

    let user_credential = repo.run(&get_user_credential).await.ok().unwrap();

    assert_eq!(user_credential.user_id, user_fixture.user_entry.id);

    let find_user_info = FindUserInfo::Qid {
        qid: &user_fixture.user_entry.qid,
    };

    let found_user_info =
        repo.run(&find_user_info).await.ok().unwrap().unwrap();

    assert_eq!(found_user_info.id, user_fixture.user_entry.id);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}

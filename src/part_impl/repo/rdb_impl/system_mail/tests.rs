// system_mail_roundtrip_reads_test_database_url(SystemMailRepo)(positive): system mail repo sends, lists, and marks mail read in the local test database.

use super::*;

use crate::model::system_mail::SystemMailEntry;
use crate::part::repo::oper::system_mail::{
    ListSystemMailInfos, MarkSystemMailRead, SendSystemMail,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};

const PREFIX: &str = "rdb-test-system-mail-domain-";

#[tokio::test]
async fn system_mail_roundtrip_reads_test_database_url() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let user_fixture = test_shared::seed_user(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let system_mail_entry = SystemMailEntry {
        id: format!("{}system-mail", PREFIX),
        receiver_id: user_fixture.user_entry.id.clone(),
        title: "RDB Mail".into(),
        content: "mail".into(),
    };

    repo.run(&SendSystemMail {
        entry: &system_mail_entry,
    })
    .await
    .ok()
    .unwrap();

    let system_mail_infos = repo
        .run(&ListSystemMailInfos {
            receiver_id: &user_fixture.user_entry.id,
            read: Some(false),
            offset: 0,
            limit: 10,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(system_mail_infos.len(), 1);

    repo.run(&MarkSystemMailRead {
        id: &system_mail_entry.id,
        user_id: &user_fixture.user_entry.id,
    })
    .await
    .ok()
    .unwrap();

    let read_system_mail_infos = repo
        .run(&ListSystemMailInfos {
            receiver_id: &user_fixture.user_entry.id,
            read: Some(true),
            offset: 0,
            limit: 10,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(read_system_mail_infos.len(), 1);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}

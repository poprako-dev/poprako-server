// system_mail_roundtrip_reads_test_database_url(SystemMailStep)(positive): system mail repo sends, lists, and marks mail read in the local test database.

use crate::model::system_mail::SystemMailForm;
use crate::part::repo::step::system_mail::SystemMailStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};

const PREFIX: &str = "rdb-test-system-mail-domain-";

#[tokio::test]
async fn system_mail_roundtrip_reads_test_database_url() {
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let user_fixture = test_shared::seed_user(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let system_mail_form = SystemMailForm {
        id: format!("{}system-mail", PREFIX),
        receiver_id: user_fixture.user_form.id.clone(),
        title: "RDB Mail".into(),
        content: "mail".into(),
    };

    Execute::execute(&repo, &SystemMailStep::send(&system_mail_form))
        .await
        .ok()
        .unwrap();

    let system_mail_infos = Execute::execute(
        &repo,
        &SystemMailStep::list_infos(
            &user_fixture.user_form.id,
            Some(false),
            0,
            10,
        ),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(system_mail_infos.len(), 1);

    Execute::execute(
        &repo,
        &SystemMailStep::mark_read(
            &system_mail_form.id,
            &user_fixture.user_form.id,
        ),
    )
    .await
    .ok()
    .unwrap();

    let read_system_mail_infos = Execute::execute(
        &repo,
        &SystemMailStep::list_infos(
            &user_fixture.user_form.id,
            Some(true),
            0,
            10,
        ),
    )
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

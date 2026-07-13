// announcement_roundtrip_reads_test_database_url(CreateAnnouncement, ListAnnouncementInfos)(positive): announcement repo creates and lists included users in the local test database.

use super::*;

use poprako_orchestra::Nucl as _;

use crate::model::announcement::{AnnouncementEntry,AnnouncementListSpec};
use crate::part::repo::oper::announcement::{
    CreateAnnouncement, ListAnnouncementInfos,
};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::RegularError;
use crate::value::announcement::AnnouncementInclOpt;

const PREFIX: &str = "rdb-test-announcement-domain-";

#[tokio::test]
async fn announcement_roundtrip_reads_test_database_url() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let announcement_entry = AnnouncementEntry {
        id: format!("{}announcement", PREFIX),
        team_id: team_fixture.team_entry.id.clone(),
        user_id: team_fixture.user_entry.id.clone(),
        title: "RDB Announcement".into(),
        content: "announcement".into(),
    };

    drive
        .coord(async |context| {
            //
            repo.step(
                context,
                &CreateAnnouncement {
                    entry: &announcement_entry,
                },
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let announcement_list_spec = AnnouncementListSpec {
        team_id: team_fixture.team_entry.id.clone(),
        incl_opt: vec![AnnouncementInclOpt::User],
        offset: 0,
        limit: 10,
    };

    let announcement_infos = repo
        .run(&ListAnnouncementInfos {
            spec: &announcement_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(announcement_infos.len(), 1);

    assert_eq!(
        announcement_infos[0].user.as_ref().unwrap().id,
        team_fixture.user_entry.id
    );

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}

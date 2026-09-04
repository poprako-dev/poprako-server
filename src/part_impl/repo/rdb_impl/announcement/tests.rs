// announcement_roundtrip_uses_testcontainer(CreateAnnouncement, GetAnnouncementInfo, ListAnnouncementInfos, UpdateAnnouncement, DeleteAnnouncement)(positive): announcement repo creates, updates, lists, and deletes in an isolated PostgreSQL container.
// announcement_roundtrip_uses_testcontainer(GetAnnouncementInfo)(negative): deleted announcement should return the expected not-found error.

use super::*;

use poprako_rdb_core::RdbCore;

use crate::model::read::spec::announcement::AnnouncementListSpec;
use crate::model::write::announcement::{AnnouncementEntry, AnnouncementRepl};
use crate::part::repo::oper::announcement::{
    CreateAnnouncement, DeleteAnnouncement, GetAnnouncementInfo,
    ListAnnouncementInfos, UpdateAnnouncement,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::announcement::AnnouncementInclOpt;

const PREFIX: &str = "rdb-test-announcement-domain-";

/// Verifies announcement roundtrip via testcontainers.
/// Verifies announcement roundtrip via testcontainers.
pub async fn announcement_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let announcement_entry = AnnouncementEntry {
        id: format!("{}announcement", PREFIX),
        team_id: team_fixture.team_entry.id.clone(),
        user_id: team_fixture.user_entry.id.clone(),
        title: "RDB Announcement".into(),
        content: "announcement".into(),
    };

    repo.run(&CreateAnnouncement {
        entry: &announcement_entry,
    })
    .await
    .ok()
    .unwrap();

    let announcement_repl = AnnouncementRepl {
        id: announcement_entry.id.clone(),
        title: "Updated RDB Announcement".into(),
        content: "updated announcement".into(),
    };

    repo.run(&GetAnnouncementInfo {
        id: &announcement_entry.id,
    })
    .await
    .ok()
    .unwrap();

    repo.run(&UpdateAnnouncement {
        update: &announcement_repl,
    })
    .await
    .ok()
    .unwrap();

    let announcement_list_spec = AnnouncementListSpec {
        team_id: team_fixture.team_entry.id.clone(),
        incl_opt: vec![AnnouncementInclOpt::User],
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    let announcement_infos = repo
        .run(&ListAnnouncementInfos {
            spec: &announcement_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(announcement_infos.len(), 1);

    assert_eq!(announcement_infos[0].title, "Updated RDB Announcement");

    assert_eq!(announcement_infos[0].content, "updated announcement");

    assert_eq!(
        announcement_infos[0].user.as_ref().unwrap().id,
        team_fixture.user_entry.id
    );

    repo.run(&GetAnnouncementInfo {
        id: &announcement_entry.id,
    })
    .await
    .ok()
    .unwrap();

    repo.run(&DeleteAnnouncement {
        id: &announcement_entry.id,
    })
    .await
    .ok()
    .unwrap();

    let missing_error = repo
        .run(&GetAnnouncementInfo {
            id: &announcement_entry.id,
        })
        .await
        .err()
        .unwrap();

    assert_expected_variant(missing_error, ExpectedVariant::Args);

    let announcement_infos = repo
        .run(&ListAnnouncementInfos {
            spec: &announcement_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert!(announcement_infos.is_empty());

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}

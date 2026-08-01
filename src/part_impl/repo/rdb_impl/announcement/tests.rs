// announcement_roundtrip_uses_testcontainer(CreateAnnouncement, ListAnnouncementInfos)(positive): announcement repo creates and lists included users in an isolated PostgreSQL container.

use super::*;

use poprako_orchestra::Nucl as _;

use crate::model::read::spec::announcement::AnnouncementListSpec;
use crate::model::write::announcement::AnnouncementEntry;
use crate::part::repo::oper::announcement::{
    CreateAnnouncement, ListAnnouncementInfos,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::BaseError;
use crate::shared::RdbCore;
use crate::value::announcement::AnnouncementInclOpt;

const PREFIX: &str = "rdb-test-announcement-domain-";

/// Verifies announcement roundtrip via testcontainers.
/// Verifies announcement roundtrip via testcontainers.
pub async fn announcement_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let nucl = RdbNucl::new(shared.clone());

    let announcement_entry = AnnouncementEntry {
        id: format!("{}announcement", PREFIX),
        team_id: team_fixture.team_entry.id.clone(),
        user_id: team_fixture.user_entry.id.clone(),
        title: "RDB Announcement".into(),
        content: "announcement".into(),
    };

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreateAnnouncement {
                entry: &announcement_entry,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
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

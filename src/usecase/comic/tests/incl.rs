use super::*;

use crate::test_util::fixture::team;
use crate::value::comic::ComicInclOpt;

fn seed_included_comic(mock: &Mock) {
    mock.seed_team(team("team--00000000001", "Team One", ""));

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_comic(comic("comic--00000000001", "workset--00000000001", 0));

    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));

    mock.seed_user(
        user("user--00000000001", "user--00000000001", "Creator"),
        invalid_credential("user--00000000001"),
    );
}

// get_info(get_info)(positive): dotted includes load the workset and its owning team.
#[tokio::test]
async fn get_info_expands_workset_team_and_creator() {
    let mock = Mock::new();

    seed_included_comic(&mock);

    let instr = GetComicInfoInstr {
        incl_opt: vec![ComicInclOpt::WorksetTeam, ComicInclOpt::Creator],
    };

    let comic_view = get_info(
        (&mock, &mock),
        token("user--00000000001"),
        "comic--00000000001".into(),
        instr,
    )
    .await
    .unwrap();

    assert_eq!(
        comic_view.workset.as_ref().unwrap().id,
        "workset--00000000001"
    );

    assert_eq!(
        comic_view.workset.as_ref().unwrap().team_id,
        "team--00000000001"
    );

    assert_eq!(comic_view.team.as_ref().unwrap().id, "team--00000000001");

    assert_eq!(comic_view.creator.as_ref().unwrap().id, "user--00000000001");
}

// get_info(get_info)(positive): omitting includes preserves the base comic response.
#[tokio::test]
async fn get_info_omits_unrequested_relations() {
    let mock = Mock::new();

    seed_included_comic(&mock);

    let comic_view = get_info(
        (&mock, &mock),
        token("user--00000000001"),
        "comic--00000000001".into(),
        GetComicInfoInstr::default(),
    )
    .await
    .unwrap();

    assert!(comic_view.workset.is_none());

    assert!(comic_view.team.is_none());

    assert!(comic_view.creator.is_none());
}

// get_info(get_info)(negative): includes do not grant access through another team.
#[tokio::test]
async fn get_info_denies_member_of_another_team() {
    let mock = Mock::new();

    seed_included_comic(&mock);

    mock.seed_member(admin_member("outsider", "team--00000000002"));

    let instr = GetComicInfoInstr {
        incl_opt: vec![ComicInclOpt::WorksetTeam],
    };

    let error = get_info(
        (&mock, &mock),
        token("outsider"),
        "comic--00000000001".into(),
        instr,
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);
}

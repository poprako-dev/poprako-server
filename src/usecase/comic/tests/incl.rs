use super::*;

use crate::test_util::fixture::team;
use crate::value::comic::ComicInclOpt;

fn seed_included_comic(mock: &Mock) {
    mock.seed_team(team("team-1", "Team One", ""));

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1", 0));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_user(
        user("user-1", "user-1", "Creator"),
        invalid_credential("user-1"),
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

    let comic_view =
        get_info((&mock, &mock), token("user-1"), "comic-1".into(), instr)
            .await
            .unwrap();

    assert_eq!(comic_view.workset.as_ref().unwrap().id, "workset-1");

    assert_eq!(comic_view.workset.as_ref().unwrap().team_id, "team-1");

    assert_eq!(comic_view.team.as_ref().unwrap().id, "team-1");

    assert_eq!(comic_view.creator.as_ref().unwrap().id, "user-1");
}

// get_info(get_info)(positive): omitting includes preserves the base comic response.
#[tokio::test]
async fn get_info_omits_unrequested_relations() {
    let mock = Mock::new();

    seed_included_comic(&mock);

    let comic_view = get_info(
        (&mock, &mock),
        token("user-1"),
        "comic-1".into(),
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

    mock.seed_member(admin_member("outsider", "team-2"));

    let instr = GetComicInfoInstr {
        incl_opt: vec![ComicInclOpt::WorksetTeam],
    };

    let error =
        get_info((&mock, &mock), token("outsider"), "comic-1".into(), instr)
            .await
            .err()
            .unwrap();

    assert_expected_variant(error, ExpectedVariant::Perm);
}

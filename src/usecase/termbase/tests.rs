// create(create)(positive): proofreader creates and normalizes a team termbase.
// create(create)(negative): admin without proofreader role cannot create a termbase.
// create(create)(negative): invalid scope is rejected without persistence.
// list_comic_infos(list_comic_infos)(positive): comic list inherits team bases and excludes sibling comic bases.
// list_comic_infos(list_comic_infos)(positive): fuzzy name does not search descriptions.
// delete(delete)(positive): deleting a termbase removes all child terms.
// delete_team_cascade(delete_team_cascade)(positive): team cascade removes direct termbases and their terms.

use super::*;

use time::OffsetDateTime;

use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::term::TermInfo;
use crate::model::termbase::TermbaseInfo;
use crate::model::workset::WorksetInfo;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::test_util::fixture::team;
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::{RoleField, RoleMask};

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn member(user_id: &str, team_id: &str, roles: RoleMask) -> MemberInfo {
    MemberInfo {
        id: format!("member-{}-{}", user_id, team_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles,
    }
}

fn workset(id: &str, team_id: &str) -> WorksetInfo {
    //
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 0,
        created_at: time,
        updated_at: time,
    }
}

fn comic(id: &str, workset_id: &str) -> ComicInfo {
    //
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: id.into(),
        author: "author".into(),
        description: None,
        cover_key: None,
        cover_uploaded: false,
        cover_version: 0,
        cover_hash: ImageHash::default(),
        cover_ext: ImageExt::Png,
        chapter_count: 0,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn termbase(
    id: &str,
    team_id: Option<&str>,
    comic_id: Option<&str>,
    name: &str,
    description: Option<&str>,
) -> TermbaseInfo {
    //
    let time = OffsetDateTime::now_utc();

    TermbaseInfo {
        id: id.into(),
        team_id: team_id.map(Into::into),
        comic_id: comic_id.map(Into::into),
        name: name.into(),
        description: description.map(Into::into),
        term_count: 0,
        creator_id: "user-1".into(),
        created_at: time,
        updated_at: time,
    }
}

fn term(id: &str, termbase_id: &str) -> TermInfo {
    //
    let time = OffsetDateTime::now_utc();

    TermInfo {
        id: id.into(),
        termbase_id: termbase_id.into(),
        source: "source".into(),
        targets: vec!["target".into()],
        comment: None,
        creator_id: "user-1".into(),
        created_at: time,
        updated_at: time,
    }
}

fn create_params() -> CreateTermbaseParams {
    CreateTermbaseParams {
        team_id: Some("team-1".into()),
        comic_id: None,
        name: "  Glossary  ".into(),
        description: Some("   ".into()),
    }
}

#[tokio::test]
async fn create_normalizes_and_persists_for_proofreader() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member(member(
        "user-1",
        "team-1",
        RoleMask::from(RoleField::PROOFREADER),
    ));

    let payload = create((&mock, &mock), token("user-1"), create_params())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.termbases.len(), 1);

    assert_eq!(snapshot.termbases[0].id, payload.id);

    assert_eq!(snapshot.termbases[0].name, "Glossary");

    assert!(snapshot.termbases[0].description.is_none());
}

#[tokio::test]
async fn create_rejects_admin_without_proofreader() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member(member(
        "user-1",
        "team-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    let error = create((&mock, &mock), token("user-1"), create_params())
        .await
        .unwrap_err();

    assert_expected_variant(error, ExpectedVariant::Perm);

    assert!(mock.snapshot().termbases.is_empty());
}

#[tokio::test]
async fn create_rejects_invalid_scope() {
    //
    let mock = Mock::new();

    let params = CreateTermbaseParams {
        team_id: Some("team-1".into()),
        comic_id: Some("comic-1".into()),
        name: "Glossary".into(),
        description: None,
    };

    let error = create((&mock, &mock), token("user-1"), params)
        .await
        .unwrap_err();

    assert_expected_variant(error, ExpectedVariant::Args);

    assert!(mock.snapshot().termbases.is_empty());
}

#[tokio::test]
async fn list_comic_infos_inherits_team_and_excludes_sibling() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member(member(
        "user-1",
        "team-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1"));

    mock.seed_comic(comic("comic-2", "workset-1"));

    mock.seed_termbase(termbase(
        "termbase-team",
        Some("team-1"),
        None,
        "Hero Team",
        None,
    ));

    mock.seed_termbase(termbase(
        "termbase-comic",
        None,
        Some("comic-1"),
        "Hero Comic",
        None,
    ));

    mock.seed_termbase(termbase(
        "termbase-sibling",
        None,
        Some("comic-2"),
        "Hero Sibling",
        None,
    ));

    let params = ListComicTermbaseInfosParams {
        comic_id: "comic-1".into(),
        fuzzy_name: Some("hero".into()),
        offset: 0,
        limit: 20,
    };

    let infos = list_comic_infos((&mock,), token("user-1"), params)
        .await
        .unwrap();

    assert_eq!(infos.len(), 2);

    assert!(infos.iter().any(|info| info.id == "termbase-team"));

    assert!(infos.iter().any(|info| info.id == "termbase-comic"));

    assert!(!infos.iter().any(|info| info.id == "termbase-sibling"));
}

#[tokio::test]
async fn list_comic_infos_does_not_search_description() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member(member(
        "user-1",
        "team-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1"));

    mock.seed_termbase(termbase(
        "termbase-1",
        Some("team-1"),
        None,
        "Glossary",
        Some("secret phrase"),
    ));

    let params = ListComicTermbaseInfosParams {
        comic_id: "comic-1".into(),
        fuzzy_name: Some("secret".into()),
        offset: 0,
        limit: 20,
    };

    let infos = list_comic_infos((&mock,), token("user-1"), params)
        .await
        .unwrap();

    assert!(infos.is_empty());
}

#[tokio::test]
async fn delete_removes_child_terms() {
    //
    let mock = Mock::new();

    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member(member(
        "user-1",
        "team-1",
        RoleMask::from(RoleField::PROOFREADER),
    ));

    let mut termbase_info =
        termbase("termbase-1", Some("team-1"), None, "Glossary", None);

    termbase_info.term_count = 1;

    mock.seed_termbase(termbase_info);

    mock.seed_term(term("term-1", "termbase-1"));

    delete((&mock, &mock), token("user-1"), "termbase-1".into())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.termbases.is_empty());

    assert!(snapshot.terms.is_empty());
}

#[tokio::test]
async fn delete_team_cascade_removes_direct_termbases_and_terms() {
    //
    let mock = Mock::new();

    mock.seed_termbase(termbase(
        "termbase-team",
        Some("team-1"),
        None,
        "Team Glossary",
        None,
    ));

    mock.seed_termbase(termbase(
        "termbase-other",
        Some("team-2"),
        None,
        "Other Glossary",
        None,
    ));

    mock.seed_term(term("term-team", "termbase-team"));

    mock.seed_term(term("term-other", "termbase-other"));

    let repo = &mock;

    mock.coord(async |context| {
        TermbaseComplex::delete_team_cascade(
            &mut step_proxy! {
                context;
                repo =>
                    for<'a> ListTermbaseInfosExcluded<'a>,
                    for<'a> GetTermbaseInfoExcluded<'a>,
                    for<'a> DeleteTerms<'a>,
                    for<'a> DeleteTermbase<'a>;
            },
            "team-1",
        )
        .await
    })
    .await
    .ok()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.termbases.len(), 1);

    assert_eq!(snapshot.termbases[0].id, "termbase-other");

    assert_eq!(snapshot.terms.len(), 1);

    assert_eq!(snapshot.terms[0].id, "term-other");
}

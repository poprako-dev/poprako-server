use super::*;

#[test]
#[expect(
    clippy::panic,
    reason = "This test fails explicitly when an expected fixture variant or assertion is violated"
)]
fn list_infos_trims_unicode_whitespace_from_fuzzy_nickname() {
    //
    let member_list_spec: MemberListSpec = ListMemberInfosInstr {
        owner_id: None,
        team_id: Some("team-1".into()),
        fuzzy_nickname: Some("\u{2003}Nick\u{00a0}".into()),
        role: None,
        incl_opt: Vec::new(),
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    }
    .try_into()
    .unwrap();

    let MemberListSpec::Team { fuzzy_nickname, .. } = member_list_spec else {
        panic!("expected team list spec");
    };

    assert_eq!(fuzzy_nickname.as_deref(), Some("Nick"));
}

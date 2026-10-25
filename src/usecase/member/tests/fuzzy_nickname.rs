use super::*;

#[test]
fn list_infos_trims_unicode_whitespace_from_fuzzy_nickname() {
    //
    let member_list_spec: MemberListSpec = ListMemberInfosInstr {
        owner_id: None,
        team_id: Some("team--00000000001".into()),
        fuzzy_nickname: Some("\u{2003}Nick\u{00a0}".into()),
        role: None,
        incl_opt: Vec::new(),
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    }
    .try_into()
    .unwrap();

    assert_eq!(member_list_spec.trimmed_fuzzy_nickname(), Some("Nick"));
}

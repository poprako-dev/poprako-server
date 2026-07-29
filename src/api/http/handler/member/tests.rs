// member_me_query(Query)(positive): missing incl should deserialize as an empty vector.
// member_me_query(Query)(positive): one incl occurrence should deserialize as one item.
// member_me_query(Query)(positive): repeated incl keys should preserve every item.

use super::*;

use axum::http::Uri;

fn parse_query(query: &str) -> MemberMeListQuery {
    //
    let uri = format!("http://localhost/members/me?{}", query)
        .parse::<Uri>()
        .unwrap();

    Query::<MemberMeListQuery>::try_from_uri(&uri).unwrap().0
}

#[test]
fn member_me_query_deserializes_missing_incl_as_empty() {
    //
    let query = parse_query("offset=0&limit=20");

    assert!(query.incl_opt.is_empty());
}

#[test]
fn member_me_query_deserializes_one_incl() {
    //
    let query = parse_query("incl=team&offset=0&limit=20");

    assert_eq!(query.incl_opt, [MemberInclOpt::Team]);
}

#[test]
fn member_me_query_deserializes_repeated_incl_keys() {
    //
    let query = parse_query("incl=team&incl=user&offset=0&limit=20");

    assert_eq!(query.incl_opt, [MemberInclOpt::Team, MemberInclOpt::User]);
}

// member_me_query(Query)(positive): missing incl should deserialize as an empty vector.
// member_me_query(Query)(positive): one incl occurrence should deserialize as one item.
// member_me_query(Query)(positive): repeated incl keys should preserve every item.

use super::*;

use axum::http::Uri;
use axum::response::IntoResponse;

// Build a local URI from the query string and reuse `Query::try_from_uri` to parse a MemberMeListQuery.
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

#[test]
fn member_me_query_accepts_public_limit_boundaries() {
    let lower = parse_query("offset=0&limit=1");

    let upper = parse_query("offset=0&limit=200");

    assert_eq!(lower.limit.get(), 1);

    assert_eq!(upper.limit.get(), 200);
}

#[test]
fn member_me_query_rejects_limit_outside_public_range() {
    for limit in [0, 201] {
        let uri = format!("http://localhost/members/me?offset=0&limit={limit}")
            .parse::<Uri>()
            .unwrap();

        let rejection =
            Query::<MemberMeListQuery>::try_from_uri(&uri).unwrap_err();

        assert_eq!(rejection.into_response().status(), StatusCode::BAD_REQUEST);
    }
}

use axum::http::Uri;
use axum_extra::extract::Query;

use crate::data::instr::comic::GetComicInfoInstr;
use crate::value::comic::ComicInclOpt;

// get_info(query)(positive): absent and repeated include parameters use the existing vocabulary.
#[test]
fn comic_query_accepts_optional_and_repeated_includes() {
    let base_uri: Uri = "/api/v1/comics/comic-1".parse().unwrap();

    let Query(base) =
        Query::<GetComicInfoInstr>::try_from_uri(&base_uri).unwrap();

    assert!(base.incl_opt.is_empty());

    let uri: Uri = "/api/v1/comics/comic-1?incl=workset.team&incl=creator"
        .parse()
        .unwrap();

    let Query(instr) = Query::<GetComicInfoInstr>::try_from_uri(&uri).unwrap();

    assert_eq!(
        instr.incl_opt,
        vec![ComicInclOpt::WorksetTeam, ComicInclOpt::Creator]
    );
}

// get_info(query)(negative): unknown includes are rejected at the HTTP boundary.
#[test]
fn comic_query_rejects_unknown_include() {
    let uri: Uri = "/api/v1/comics/comic-1?incl=missing".parse().unwrap();

    assert!(Query::<GetComicInfoInstr>::try_from_uri(&uri).is_err());
}

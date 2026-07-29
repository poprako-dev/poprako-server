use super::*;

use crate::value::assignment::AssignmentInclOpt;
use crate::value::member::MemberInclOpt;

#[test]
fn identity_opts_keep_the_requested_order_and_deduplicate() {
    //
    let incl_opts = [
        MemberInclOpt::Team,
        MemberInclOpt::User,
        MemberInclOpt::Team,
    ];

    assert_eq!(
        expand_incl_opts(&incl_opts),
        [MemberInclOpt::Team, MemberInclOpt::User],
    );
}

#[test]
fn deep_paths_expand_dependencies_once_in_order() {
    //
    let incl_opts = [
        AssignmentInclOpt::ChapterComicWorksetTeam,
        AssignmentInclOpt::ChapterCreator,
        AssignmentInclOpt::ChapterComicWorkset,
        AssignmentInclOpt::ChapterComicCreator,
    ];

    assert_eq!(
        expand_incl_opts(&incl_opts),
        [
            AssignmentInclOpt::Chapter,
            AssignmentInclOpt::ChapterComic,
            AssignmentInclOpt::ChapterComicWorkset,
            AssignmentInclOpt::ChapterComicWorksetTeam,
            AssignmentInclOpt::ChapterCreator,
            AssignmentInclOpt::ChapterComicCreator,
        ],
    );
}

#[test]
fn empty_input_has_no_include_plan() {
    assert!(expand_incl_opts::<MemberInclOpt>(&[]).is_empty());
}

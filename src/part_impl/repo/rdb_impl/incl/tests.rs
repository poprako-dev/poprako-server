#[allow(unused_imports)]
use super::*;

use crate::model::user::UserInfo;
use crate::part_impl::repo::rdb_impl::incl::framework::{
    BatchByIds, Incl, UserByIds, populate,
};
use crate::part_impl::shared::RdbConn;
use crate::result::BaseRest;
use crate::value::incl::{InclOpt, expand_incl_opts};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Include options used to compile-test macro expansion.
pub(super) enum MacroTestInclOpt {
    /// Tests direct field paths.
    Direct,

    /// Tests nested optional paths.
    Nested,

    /// Tests distinct resolve and inject paths.
    Asymmetric,

    /// Tests the resolve escape hatch.
    ResolveEscape,

    /// Tests the inject escape hatch.
    InjectEscape,

    /// Tests both escape hatches together.
    BothEscapes,
}

impl InclOpt for MacroTestInclOpt {
    fn path(self) -> &'static [Self] {
        match self {
            //
            Self::Direct => &[Self::Direct],

            Self::Nested => &[Self::Nested],

            Self::Asymmetric => &[Self::Asymmetric],

            Self::ResolveEscape => &[Self::ResolveEscape],

            Self::InjectEscape => &[Self::InjectEscape],

            Self::BothEscapes => &[Self::BothEscapes],
        }
    }
}

struct MacroNestedInfo {
    //
    user_id: String,
    user: Option<UserInfo>,
}

/// Owner fixture used to compile-test macro expansion.
pub(super) struct MacroTestInfo {
    //
    /// User ID used in the preloadable macro expansion test.
    user_id: String,
    /// Optional user info for testing nested preloadable expansion.
    user: Option<UserInfo>,
    /// Optional nested info for testing multi-level preloadable expansion.
    nested: Option<MacroNestedInfo>,
}

preloadable! {
    owner: MacroTestInfo,
    opt: MacroTestInclOpt,
    populate: populate_macro_test_incls,
    variants: {
        Direct => UserByIds {
            resolve: path [] => user_id,
            inject: path [] => user,
        },
        Nested => UserByIds {
            resolve: path [nested] => user_id,
            inject: path [nested] => user,
        },
        Asymmetric => UserByIds {
            resolve: path [nested] => user_id,
            inject: path [] => user,
        },
        ResolveEscape => UserByIds {
            resolve: with |owner| Some(owner.user_id.as_str()),
            inject: path [] => user,
        },
        InjectEscape => UserByIds {
            resolve: path [] => user_id,
            inject: with |owner, related| owner.user = related,
        },
        BothEscapes => UserByIds {
            resolve: with |owner| owner.nested.as_ref().map(|nested| nested.user_id.as_str()),
            inject: with |owner, related| owner.user = related,
        },
    },
}

#[test]
fn preloadable_paths_and_escape_hatches_compile_and_resolve() {
    //
    let _ = populate_macro_test_incls;

    let mut macro_test_info = MacroTestInfo {
        user_id: "user-direct".into(),
        user: None,
        nested: Some(MacroNestedInfo {
            user_id: "user-nested".into(),
            user: None,
        }),
    };

    assert_eq!(Direct::resolve_key(&macro_test_info), Some("user-direct"));

    assert_eq!(Nested::resolve_key(&macro_test_info), Some("user-nested"));

    assert_eq!(
        Asymmetric::resolve_key(&macro_test_info),
        Some("user-nested")
    );

    assert_eq!(
        ResolveEscape::resolve_key(&macro_test_info),
        Some("user-direct")
    );

    assert_eq!(
        InjectEscape::resolve_key(&macro_test_info),
        Some("user-direct")
    );

    assert_eq!(
        BothEscapes::resolve_key(&macro_test_info),
        Some("user-nested")
    );

    Direct::inject(&mut macro_test_info, None);

    Nested::inject(&mut macro_test_info, None);

    Asymmetric::inject(&mut macro_test_info, None);

    ResolveEscape::inject(&mut macro_test_info, None);

    InjectEscape::inject(&mut macro_test_info, None);

    BothEscapes::inject(&mut macro_test_info, None);

    assert!(macro_test_info.user.is_none());

    assert!(macro_test_info.nested.unwrap().user.is_none());
}

#[test]
fn macro_test_incl_opts_expand_without_dependencies() {
    //
    let incl_opts = [MacroTestInclOpt::Direct, MacroTestInclOpt::Direct];

    assert_eq!(expand_incl_opts(&incl_opts), [MacroTestInclOpt::Direct]);
}

#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::{UpdateUserPasswordBody, UserInfoObj};
use crate::data::view::obj_url::ObjUrlView;
use crate::data::view::user::UserInfoView;

// Builds a user profile view with the given avatar URLs.
fn user_info_view(
    avatar_url: Option<ObjUrlView>,
    avatar_thumbnail_url: Option<ObjUrlView>,
) -> UserInfoView {
    UserInfoView {
        id: "user--00000000001".into(),
        nickname: "User".into(),
        qid: "qid-1".into(),
        avatar_url,
        avatar_thumbnail_url,
        is_sadmin: false,
        last_active_at: 0,
        created_at: 0,
        updated_at: 0,
    }
}

// Checks compact and pretty Debug on a credential-bearing wire body.
fn check_sensitive<T: Debug + DeserializeOwned>(
    value: Value,
) -> serde_json::Result<T> {
    //
    let body: T = serde_json::from_value(value)?;

    for output in [format!("{body:?}"), format!("{body:#?}")] {
        //
        assert_eq!(output.contains("private-sentinel"), cfg!(debug_assertions));

        assert_eq!(output.contains("[REDACTED]"), !cfg!(debug_assertions));

        assert!(!output.contains("\"[REDACTED]\""));
    }

    Ok(body)
}

#[test]
#[expect(
    clippy::panic,
    reason = "This test fails explicitly when an expected fixture variant or assertion is violated"
)]
fn user_info_obj_omits_absent_avatar_urls() {
    //
    let obj = UserInfoObj::from(user_info_view(None, None));

    let serialized = serde_json::to_value(obj).unwrap();

    let Value::Object(object) = serialized else {
        panic!("user info object must serialize as an object");
    };

    assert!(!object.contains_key("avatar_url"));

    assert!(!object.contains_key("avatar_thumbnail_url"));
}

#[test]
fn user_info_obj_converts_avatar_urls_into_owned_strings() {
    //
    let obj = UserInfoObj::from(user_info_view(
        Some(ObjUrlView::from(String::from("https://obj.test/avatar"))),
        Some(ObjUrlView::from(String::from(
            "https://obj.test/avatar-thumb",
        ))),
    ));

    assert_eq!(obj.avatar_url.as_deref(), Some("https://obj.test/avatar"));

    assert_eq!(
        obj.avatar_thumbnail_url.as_deref(),
        Some("https://obj.test/avatar-thumb"),
    );

    let serialized = serde_json::to_value(&obj).unwrap();

    assert_eq!(
        serialized.get("avatar_url").and_then(Value::as_str),
        Some("https://obj.test/avatar"),
    );
}

#[test]
fn password_body_redacts_its_credentials() {
    //
    let body = check_sensitive::<UpdateUserPasswordBody>(json!({
        "current_password": "private-sentinel",
        "new_password": "private-sentinel",
    }))
    .unwrap();

    assert_eq!(body.current_password, "private-sentinel");

    assert_eq!(body.new_password, "private-sentinel");
}

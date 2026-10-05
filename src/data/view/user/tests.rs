#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::UserInfoView;

#[test]
#[expect(
    clippy::panic,
    reason = "This test fails explicitly when an expected fixture variant or assertion is violated"
)]
fn user_info_view_omits_absent_avatar_urls() {
    let user_info_view = UserInfoView {
        id: "user-1".into(),
        nickname: "User".into(),
        qid: "qid-1".into(),
        avatar_url: None,
        avatar_thumbnail_url: None,
        is_sadmin: false,
        last_active_at: 0,
        created_at: 0,
        updated_at: 0,
    };

    let serialized = serde_json::to_value(user_info_view).unwrap();

    let serde_json::Value::Object(object) = serialized else {
        panic!("user info value must serialize as an object");
    };

    assert!(!object.contains_key("avatar_url"));
    assert!(!object.contains_key("avatar_thumbnail_url"));
}

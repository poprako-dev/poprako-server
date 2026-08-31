use super::TeamInfoView;

#[test]
fn team_info_view_omits_absent_avatar_urls() {
    let team_info_view = TeamInfoView {
        id: "team-1".into(),
        name: "Team".into(),
        description: String::new(),
        avatar_url: None,
        avatar_thumbnail_url: None,
        created_at: 0,
        updated_at: 0,
    };

    let serialized = serde_json::to_value(team_info_view).unwrap();

    let serde_json::Value::Object(object) = serialized else {
        panic!("team info value must serialize as an object");
    };

    assert!(!object.contains_key("avatar_url"));
    assert!(!object.contains_key("avatar_thumbnail_url"));
}

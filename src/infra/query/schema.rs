// @generated automatically by Diesel CLI.

diesel::table! {
    t_user (f_id) {
        f_id -> Text,
        f_nickname -> Text,
        f_qid -> Text,
        f_is_sadmin -> Bool,
        f_avatar_key -> Nullable<Text>,
        f_avatar_source -> Nullable<Text>,
        f_avatar_uploaded -> Bool,
        f_password_hash -> Text,
        f_last_active_at -> Timestamptz,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

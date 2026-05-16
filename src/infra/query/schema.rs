// @generated automatically by Diesel CLI.

diesel::table! {
    tbl_user (id) {
        id -> Text,
        nickname -> Text,
        qid -> Text,
        is_sadmin -> Bool,
        avatar_key -> Nullable<Text>,
        avatar_source -> Nullable<Text>,
        avatar_uploaded -> Bool,
        password_hash -> Text,
        last_active_at -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

// @generated automatically by Diesel CLI.

diesel::table! {
    t_member (f_id) {
        f_id -> Text,
        f_user_id -> Text,
        f_user_nickname -> Text,
        f_team_id -> Text,
        f_assigned_raw_provider_at -> Nullable<Timestamptz>,
        f_assigned_translator_at -> Nullable<Timestamptz>,
        f_assigned_proofreader_at -> Nullable<Timestamptz>,
        f_assigned_typesetter_at -> Nullable<Timestamptz>,
        f_assigned_redrawer_at -> Nullable<Timestamptz>,
        f_assigned_reviewer_at -> Nullable<Timestamptz>,
        f_assigned_publisher_at -> Nullable<Timestamptz>,
        f_assigned_admin_at -> Nullable<Timestamptz>,
        f_assigned_assistant_at -> Nullable<Timestamptz>,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_member_invitation (f_id) {
        f_id -> Text,
        f_inviter_id -> Text,
        f_team_id -> Text,
        f_invitee_qid -> Text,
        f_invitation_code -> Text,
        f_pending -> Bool,
        f_role_mask -> Int8,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

diesel::table! {
    t_system_mail (f_id) {
        f_id -> Text,
        f_receiver_id -> Text,
        f_title -> Text,
        f_content -> Text,
        f_read -> Bool,
        f_created_at -> Timestamptz,
    }
}

diesel::table! {
    t_team (f_id) {
        f_id -> Text,
        f_name -> Text,
        f_description -> Nullable<Text>,
        f_avatar_key -> Nullable<Text>,
        f_avatar_uploaded -> Bool,
        f_workset_next_index -> Int4,
        f_created_at -> Timestamptz,
        f_updated_at -> Timestamptz,
    }
}

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

diesel::joinable!(t_member -> t_team (f_team_id));
diesel::joinable!(t_member -> t_user (f_user_id));
diesel::joinable!(t_member_invitation -> t_team (f_team_id));
diesel::joinable!(t_member_invitation -> t_user (f_inviter_id));
diesel::joinable!(t_system_mail -> t_user (f_receiver_id));

diesel::allow_tables_to_appear_in_same_query!(
    t_member,
    t_member_invitation,
    t_system_mail,
    t_team,
    t_user,
);

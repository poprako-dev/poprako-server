INSERT INTO t_user (
    f_id,
    f_nickname,
    f_qid,
    f_avatar_key,
    f_avatar_source,
    f_avatar_uploaded,
    f_avatar_version,
    f_password_hash
) VALUES (
    'user-bridgefixture',
    'Bridge Fixture',
    'bridge-fixture',
    'avatars/user.WEBP',
    'fixture',
    TRUE,
    7,
    '$argon2id$v=19$m=1,t=1,p=1$fixture$fixture'
);

INSERT INTO t_team (
    f_id,
    f_name,
    f_description,
    f_avatar_key,
    f_avatar_uploaded,
    f_avatar_version,
    f_workset_next_index
) VALUES (
    'team-bridgefixture',
    'Bridge Fixture',
    'bridge fixture team',
    'avatars/team.unsupported',
    TRUE,
    8,
    2
);

INSERT INTO t_member_invitation (
    f_id,
    f_inviter_id,
    f_team_id,
    f_invitee_qid,
    f_code,
    f_role_mask
) VALUES (
    'member-invitation-bridgefixture',
    'user-bridgefixture',
    'team-bridgefixture',
    'invitee-fixture',
    'member-code-fixture',
    3
);

INSERT INTO t_member (
    f_id,
    f_user_id,
    f_user_nickname,
    f_team_id,
    f_assigned_translator_at
) VALUES (
    'member-bridgefixture',
    'user-bridgefixture',
    'Bridge Fixture',
    'team-bridgefixture',
    '2026-07-20T00:00:00Z'
);

INSERT INTO t_system_mail (
    f_id,
    f_receiver_id,
    f_title,
    f_content
) VALUES (
    'system-mail-bridgefixture',
    'user-bridgefixture',
    'fixture title',
    'fixture content'
);

INSERT INTO t_workset (
    f_id,
    f_team_id,
    f_index,
    f_name,
    f_comic_count,
    f_comic_next_index
) VALUES (
    'workset-bridgefixture',
    'team-bridgefixture',
    1,
    'fixture workset',
    1,
    2
);

INSERT INTO t_comic (
    f_id,
    f_workset_id,
    f_index,
    f_title,
    f_author,
    f_composed_title,
    f_cover_key,
    f_cover_uploaded,
    f_cover_version,
    f_chapter_count,
    f_chapter_next_index,
    f_creator_id
) VALUES (
    'comic-bridgefixture',
    'workset-bridgefixture',
    1,
    'fixture comic',
    'fixture author',
    'fixture comic fixture author',
    'covers/comic.JPEG',
    TRUE,
    9,
    1,
    2,
    'user-bridgefixture'
);

INSERT INTO t_chapter (
    f_id,
    f_comic_id,
    f_index,
    f_subtitle,
    f_page_count,
    f_total_unit_count,
    f_translated_unit_count,
    f_proofread_unit_count,
    f_creator_id
) VALUES (
    'chapter-bridgefixture',
    'comic-bridgefixture',
    1,
    'fixture chapter',
    1,
    1,
    1,
    1,
    'user-bridgefixture'
);

INSERT INTO t_page (
    f_id,
    f_chapter_id,
    f_index,
    f_image_key,
    f_image_uploaded,
    f_image_version,
    f_image_hash,
    f_image_byte_length,
    f_image_extension,
    f_total_unit_count,
    f_translated_unit_count,
    f_proofread_unit_count
) VALUES (
    'page-bridgefixture',
    'chapter-bridgefixture',
    1,
    'pages/page.TIFF',
    TRUE,
    10,
    decode(repeat('ab', 32), 'hex'),
    1024,
    'png',
    1,
    1,
    1
);

INSERT INTO t_unit (
    f_id,
    f_page_id,
    f_index,
    f_is_bubble,
    f_is_proofread,
    f_x_coord,
    f_y_coord,
    f_translated_text,
    f_last_translator_id,
    f_proofread_text,
    f_last_proofreader_id
) VALUES (
    'unit-bridgefixture',
    'page-bridgefixture',
    1,
    TRUE,
    TRUE,
    12.5,
    24.5,
    'fixture translation',
    'user-bridgefixture',
    'fixture proofread',
    'user-bridgefixture'
);

INSERT INTO t_assignment (
    f_id,
    f_chapter_id,
    f_user_id,
    f_assigned_translator_at
) VALUES (
    'assignment-bridgefixture',
    'chapter-bridgefixture',
    'user-bridgefixture',
    '2026-07-20T01:00:00Z'
);

INSERT INTO t_assignment_invitation (
    f_id,
    f_chapter_id,
    f_inviter_id,
    f_invitee_qid,
    f_code,
    f_role_mask
) VALUES (
    'assignment-invitation-bridgefixture',
    'chapter-bridgefixture',
    'user-bridgefixture',
    'assignment-invitee-fixture',
    'assignment-code-fixture',
    2
);

INSERT INTO t_announcement (
    f_id,
    f_team_id,
    f_user_id,
    f_title,
    f_content
) VALUES (
    'announcement-bridgefixture',
    'team-bridgefixture',
    'user-bridgefixture',
    'fixture announcement',
    'fixture announcement content'
);

INSERT INTO t_comment (
    f_id,
    f_team_id,
    f_user_id,
    f_content
) VALUES (
    'comment-bridgefixture',
    'team-bridgefixture',
    'user-bridgefixture',
    'fixture comment'
);

INSERT INTO t_termbase (
    f_id,
    f_team_id,
    f_name,
    f_term_count,
    f_creator_id
) VALUES (
    'termbase-bridgefixture',
    'team-bridgefixture',
    'fixture termbase',
    1,
    'user-bridgefixture'
);

INSERT INTO t_term (
    f_id,
    f_termbase_id,
    f_source,
    f_targets,
    f_comment,
    f_creator_id
) VALUES (
    'term-bridgefixture',
    'termbase-bridgefixture',
    'fixture source',
    ARRAY['fixture target'],
    'fixture term comment',
    'user-bridgefixture'
);

INSERT INTO t_comic_archive (
    f_id,
    f_team_id,
    f_archived_payload,
    f_archiver_id,
    f_created_at
) VALUES (
    'comic-archive-bridgefixture',
    'team-bridgefixture',
    '{}',
    'user-bridgefixture',
    '2026-07-20T02:00:00Z'
);

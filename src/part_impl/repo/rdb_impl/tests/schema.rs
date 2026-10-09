// all_application_table_columns_match_generated_schema(schema)(positive): typed projections and catalog columns match every generated application table.

use std::collections::{BTreeMap, BTreeSet};

use diesel::TextExpressionMethods as _;
use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;

use poprako_rdb_core::RdbCore;

use crate::part_impl::repo::rdb_impl::schema::t_announcement;
use crate::part_impl::repo::rdb_impl::schema::t_assignment;
use crate::part_impl::repo::rdb_impl::schema::t_assignment_invitation;
use crate::part_impl::repo::rdb_impl::schema::t_chapter;
use crate::part_impl::repo::rdb_impl::schema::t_chapter_artwork;
use crate::part_impl::repo::rdb_impl::schema::t_chapter_workflow_record;
use crate::part_impl::repo::rdb_impl::schema::t_comic;
use crate::part_impl::repo::rdb_impl::schema::t_comic_archive;
use crate::part_impl::repo::rdb_impl::schema::t_comic_cover;
use crate::part_impl::repo::rdb_impl::schema::t_comment;
use crate::part_impl::repo::rdb_impl::schema::t_issue;
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::part_impl::repo::rdb_impl::schema::t_member;
use crate::part_impl::repo::rdb_impl::schema::t_member_invitation;
use crate::part_impl::repo::rdb_impl::schema::t_obj_prom_task;
use crate::part_impl::repo::rdb_impl::schema::t_page;
use crate::part_impl::repo::rdb_impl::schema::t_page_image;
use crate::part_impl::repo::rdb_impl::schema::t_page_raw_ident;
use crate::part_impl::repo::rdb_impl::schema::t_system_mail;
use crate::part_impl::repo::rdb_impl::schema::t_team;
use crate::part_impl::repo::rdb_impl::schema::t_team_avatar;
use crate::part_impl::repo::rdb_impl::schema::t_term;
use crate::part_impl::repo::rdb_impl::schema::t_termbase;
use crate::part_impl::repo::rdb_impl::schema::t_unit;
use crate::part_impl::repo::rdb_impl::schema::t_unit_save;
use crate::part_impl::repo::rdb_impl::schema::t_user;
use crate::part_impl::repo::rdb_impl::schema::t_user_avatar;
use crate::part_impl::repo::rdb_impl::schema::t_workset;

// PostgreSQL's catalog view is queried through typed schema expressions.
diesel::table! {
    information_schema.columns (table_schema, table_name, column_name) {
        table_schema -> Text,
        table_name -> Text,
        column_name -> Text,
    }
}

fn parse_generated_schema() -> BTreeMap<String, BTreeSet<String>> {
    let mut tables = BTreeMap::new();

    let mut lines = include_str!("../schema.rs").lines();

    while let Some(line) = lines.next() {
        if line.trim() != "diesel::table! {" {
            continue;
        }

        let table_line = lines
            .find(|candidate| !candidate.trim().is_empty())
            .expect("generated table declaration should contain a table name")
            .trim();

        let table_name = table_line
            .split_once(' ')
            .map(|(name, _)| name)
            .expect("generated table name should precede its primary key");

        let mut column_names = BTreeSet::new();

        for column_line in lines.by_ref() {
            let column_line = column_line.trim();

            if column_line == "}" {
                break;
            }

            let Some((column_name, _)) = column_line.split_once(" -> ") else {
                continue;
            };

            column_names.insert(column_name.to_owned());
        }

        assert!(!column_names.is_empty());

        tables.insert(table_name.to_owned(), column_names);
    }

    assert!(!tables.is_empty());

    tables
}

/// Verifies every generated table projection and the complete application catalog.
pub async fn all_application_table_columns_match_generated_schema(
    shared: RdbCore,
) {
    let expected_tables = parse_generated_schema();

    let mut conn = shared.get().await.unwrap();

    let mut probed_tables = BTreeSet::new();

    macro_rules! probe_tables {
        ($($table:ident),+ $(,)?) => {
            $(
                $table::table
                    .select($table::all_columns)
                    .limit(0)
                    .execute(&mut conn)
                    .await
                    .unwrap();

                probed_tables.insert(stringify!($table).to_owned());
            )+
        };
    }

    probe_tables!(
        t_announcement,
        t_assignment,
        t_assignment_invitation,
        t_chapter,
        t_chapter_artwork,
        t_chapter_workflow_record,
        t_comic,
        t_comic_archive,
        t_comic_cover,
        t_comment,
        t_local_message,
        t_member,
        t_member_invitation,
        t_obj_prom_task,
        t_page,
        t_issue,
        t_page_image,
        t_page_raw_ident,
        t_system_mail,
        t_team,
        t_team_avatar,
        t_term,
        t_termbase,
        t_unit,
        t_unit_save,
        t_user,
        t_user_avatar,
        t_workset,
    );

    assert_eq!(probed_tables, expected_tables.keys().cloned().collect());

    let catalog_columns = columns::table
        .filter(columns::table_schema.eq("public"))
        .filter(columns::table_name.like("t\\_%"))
        .select((columns::table_name, columns::column_name))
        .load::<(String, String)>(&mut conn)
        .await
        .unwrap();

    let actual_tables = catalog_columns.into_iter().fold(
        BTreeMap::<String, BTreeSet<String>>::new(),
        |mut tables, (table_name, column_name)| {
            tables.entry(table_name).or_default().insert(column_name);

            tables
        },
    );

    assert_eq!(actual_tables, expected_tables);
}

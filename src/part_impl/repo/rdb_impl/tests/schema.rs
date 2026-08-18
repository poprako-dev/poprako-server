// all_application_table_columns_match_generated_schema(schema)(positive): every generated application table column is selectable and exactly matches PostgreSQL.

use std::collections::{BTreeMap, BTreeSet};

use diesel::QueryableByName;
use diesel::sql_types::{Bool, Text};
use diesel_async::RunQueryDsl;

use crate::shared::RdbCore;

#[derive(QueryableByName)]
struct CatalogColumn {
    #[diesel(sql_type = Text)]
    table_name: String,
    #[diesel(sql_type = Text)]
    column_name: String,
}

#[derive(QueryableByName)]
struct SelectProbe {
    #[diesel(sql_type = Bool)]
    selected: bool,
}

pub async fn all_application_table_columns_match_generated_schema(
    shared: RdbCore,
) {
    //
    let expected_tables = parse_generated_schema();

    let mut conn = shared.get().await.unwrap();

    for (table_name, column_names) in &expected_tables {
        //
        let selected_columns = column_names
            .iter()
            .map(|column_name| format!("\"{}\"", column_name))
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            "SELECT \"marker\".\"selected\" AS \"selected\" \
             FROM (VALUES (TRUE)) AS \"marker\"(\"selected\") \
             LEFT JOIN LATERAL (\
                 SELECT {selected_columns} FROM \"{table_name}\" LIMIT 0\
             ) AS \"all_columns\" ON TRUE \
             LIMIT 1",
        );

        let probes = diesel::sql_query(query)
            .load::<SelectProbe>(&mut conn)
            .await
            .unwrap_or_else(|err| {
                panic!(
                    "failed to select every column from {}: {}",
                    table_name, err,
                )
            });

        assert_eq!(probes.len(), 1);

        assert!(probes[0].selected);
    }

    let catalog_columns = diesel::sql_query(
        "SELECT table_name, column_name \
         FROM information_schema.columns \
         WHERE table_schema = 'public' AND LEFT(table_name, 2) = 't_' \
         ORDER BY table_name, column_name",
    )
    .load::<CatalogColumn>(&mut conn)
    .await
    .unwrap();

    let actual_tables = catalog_columns.into_iter().fold(
        BTreeMap::<String, BTreeSet<String>>::new(),
        |mut tables, catalog_column| {
            //
            tables
                .entry(catalog_column.table_name)
                .or_default()
                .insert(catalog_column.column_name);

            tables
        },
    );

    assert_eq!(actual_tables, expected_tables);
}

fn parse_generated_schema() -> BTreeMap<String, BTreeSet<String>> {
    //
    let mut tables = BTreeMap::new();

    let mut lines = include_str!("../schema.rs").lines();

    while let Some(line) = lines.next() {
        //
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
            //
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

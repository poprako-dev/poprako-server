use super::*;

use diesel::QueryableByName;
use diesel::sql_types::BigInt;
use diesel_async::RunQueryDsl as _;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner as _;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt as _};

#[derive(QueryableByName)]
struct SeedCounts {
    #[diesel(sql_type = BigInt)]
    user_count: i64,
    #[diesel(sql_type = BigInt)]
    team_count: i64,
    #[diesel(sql_type = BigInt)]
    member_count: i64,
}

async fn empty_database() -> (ContainerAsync<Postgres>, String) {
    let container = Postgres::default()
        .with_tag("18-alpine")
        .start()
        .await
        .expect("test PostgreSQL container should start");

    let host = container
        .get_host()
        .await
        .expect("test PostgreSQL host should be available");

    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("test PostgreSQL port should be available");

    let database_url =
        format!("postgres://postgres:postgres@{}:{}/postgres", host, port);

    (container, database_url)
}

#[tokio::test]
async fn prepare_creates_schema_and_is_repeatable() {
    let (_container, database_url) = empty_database().await;
    let core = RdbCore::from_database_url(&database_url)
        .expect("test PostgreSQL pool should be built");

    core.prepare()
        .await
        .expect("first database preparation should succeed");

    core.prepare()
        .await
        .expect("repeated database preparation should succeed");

    let mut conn = core
        .get()
        .await
        .expect("test PostgreSQL connection should be available");
    let seed_counts = diesel::sql_query(
        "SELECT \
         (SELECT COUNT(*) FROM t_user WHERE f_id = 'user-11111111111') \
             AS user_count, \
         (SELECT COUNT(*) FROM t_team WHERE f_id = 'team-11111111111') \
             AS team_count, \
         (SELECT COUNT(*) FROM t_member WHERE f_id = 'member-11111111111') \
             AS member_count",
    )
    .get_result::<SeedCounts>(&mut conn)
    .await
    .expect("bootstrap rows should be queryable");

    assert_eq!(seed_counts.user_count, 1);
    assert_eq!(seed_counts.team_count, 1);
    assert_eq!(seed_counts.member_count, 1);
}

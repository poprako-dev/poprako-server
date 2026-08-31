use diesel::{Connection as _, PgConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness as _};
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner as _;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt as _};

use poprako_rdb_core::RdbCore;

const MIGRATIONS: EmbeddedMigrations =
    diesel_migrations::embed_migrations!("migrations");

pub struct TestRdb {
    //
    core: RdbCore,
    _container: ContainerAsync<Postgres>,
}

impl TestRdb {
    pub fn core(&self) -> RdbCore {
        self.core.clone()
    }
}

fn run_migrations(database_url: &str) {
    //
    let mut conn = PgConnection::establish(database_url)
        .expect("test PostgreSQL connection should be established");

    conn.run_pending_migrations(MIGRATIONS)
        .expect("test PostgreSQL migrations should succeed");
}

pub async fn start() -> TestRdb {
    //
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

    run_migrations(&database_url);

    let core = RdbCore::from_database_url(&database_url)
        .expect("test PostgreSQL pool should be built");

    TestRdb {
        core,
        _container: container,
    }
}

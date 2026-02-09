use aave_v3_tracker::db::connection::DbPool;
use diesel::Connection;
use diesel::PgConnection;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use testcontainers::ContainerAsync;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;

static TEST_CONTAINER: OnceCell<ContainerAsync<Postgres>> = OnceCell::const_new();
static TEST_POOL: OnceCell<DbPool> = OnceCell::const_new();

pub async fn get_test_pool() -> DbPool {
    TEST_POOL
        .get_or_init(|| async {
            let container = TEST_CONTAINER
                .get_or_init(|| async {
                    Postgres::default()
                        .with_tag("15-alpine")
                        .start()
                        .await
                        .expect("Failed to start postgres container")
                })
                .await;

            let port = container.get_host_port_ipv4(5432).await.unwrap();
            let connection_string =
                format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port);

            run_migrations(&connection_string);

            let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(&connection_string);
            Pool::builder(config)
                .max_size(5)
                .build()
                .expect("Failed to create pool")
        })
        .await
        .clone()
}

fn run_migrations(database_url: &str) {
    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    let mut conn = PgConnection::establish(database_url).expect("Failed to connect for migrations");

    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");
}

pub struct TestDb {
    pool: DbPool,
}

impl TestDb {
    pub async fn new() -> Self {
        let pool = get_test_pool().await;
        Self { pool }
    }

    pub async fn conn(
        &self,
    ) -> diesel_async::pooled_connection::deadpool::Object<AsyncPgConnection> {
        self.pool.get().await.expect("Failed to get connection")
    }

    pub fn pool(&self) -> DbPool {
        self.pool.clone()
    }
}

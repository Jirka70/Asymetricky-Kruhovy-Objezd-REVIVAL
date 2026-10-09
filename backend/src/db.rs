use diesel::{
    Connection,
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool},
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::time::Duration;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn connect(url: &str) -> anyhow::Result<PgConnection> {
    // Avoid propagating connection errors that may contain credentials.
    PgConnection::establish(url).map_err(|_| {
        anyhow::anyhow!("Database connection failed; check DATABASE_URL and PostgreSQL")
    })
}

pub fn pool(url: &str, size: u32) -> anyhow::Result<DbPool> {
    anyhow::ensure!(size > 0, "DB_POOL_SIZE must be greater than zero");
    Pool::builder()
        .max_size(size)
        .connection_timeout(Duration::from_secs(5))
        .build(ConnectionManager::new(url))
        .map_err(|_| anyhow::anyhow!("Could not initialize database pool"))
}

pub fn migrate(connection: &mut PgConnection) -> anyhow::Result<usize> {
    connection
        .run_pending_migrations(MIGRATIONS)
        .map(|versions| versions.len())
        .map_err(|error| anyhow::anyhow!("Migration failed: {error}"))
}

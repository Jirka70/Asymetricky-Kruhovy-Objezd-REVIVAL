use anyhow::Context;
use diesel_migrations::MigrationHarness;
use obor_backend::{api, db};
use std::{env, net::SocketAddr};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("obor_backend=info")),
        )
        .init();
    let args: Vec<_> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("serve");
    if command == "--help" || command == "-h" {
        println!(
            "Usage: obor-backend [serve|migrate|migrate-status]\nConfiguration: DATABASE_URL, BIND_ADDRESS (127.0.0.1:8000), DB_POOL_SIZE (8)"
        );
        return Ok(());
    }
    anyhow::ensure!(
        args.len() <= 1 && matches!(command, "serve" | "migrate" | "migrate-status"),
        "Use --help for valid commands"
    );
    let url =
        env::var("DATABASE_URL").context("Set DATABASE_URL in the environment or backend/.env")?;
    let mut connection = db::connect(&url)?;
    if command == "migrate" {
        println!("Applied {} migration(s)", db::migrate(&mut connection)?);
        return Ok(());
    }
    let pending = connection
        .pending_migrations(db::MIGRATIONS)
        .map_err(|error| anyhow::anyhow!("Cannot inspect migrations: {error}"))?;
    if command == "migrate-status" {
        for migration in &pending {
            println!("Pending: {}", migration.name());
        }
        println!("{} pending migration(s)", pending.len());
        return Ok(());
    }
    anyhow::ensure!(
        pending.is_empty(),
        "Database has pending migrations; run cargo run -- migrate first"
    );
    drop(connection);
    let size: u32 = env::var("DB_POOL_SIZE")
        .unwrap_or_else(|_| "8".into())
        .parse()
        .context("DB_POOL_SIZE must be a positive integer")?;
    let address: SocketAddr = env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8000".into())
        .parse()
        .context("BIND_ADDRESS must be an IP address and port")?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "Backend listening");
    axum::serve(listener, api::router(db::pool(&url, size)?))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

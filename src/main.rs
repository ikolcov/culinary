use anyhow::Context;
use clap::Parser;

use culinary::config::Config;
use culinary::http;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let config = Config::parse();

    let db = sqlx::postgres::PgPoolOptions::new()
        .max_connections(50)
        .connect(&config.database_url)
        .await
        .context("could not connect to database_url")?;

    sqlx::migrate!().run(&db).await?;

    http::serve(config, db).await?;

    Ok(())
}

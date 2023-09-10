use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash};
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

use crate::config::Config;

#[derive(Clone)]
pub(crate) struct ApiContext {
    config: Arc<Config>,
    db: PgPool,
}

pub use crate::error::Error;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub async fn serve(config: Config, db: PgPool) -> anyhow::Result<()> {
    let api_context = ApiContext {
        config: Arc::new(config),
        db,
    };
    let router = Router::new()
        .route("/users", post(create_user))
        .layer(TraceLayer::new_for_http())
        .with_state(api_context);
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8080));
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .context("error running HTTP server")
}

#[derive(Deserialize)]
struct NewUser {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct User {
    user_id: String,
    email: String,
}

async fn create_user(ctx: State<ApiContext>, Json(req): Json<NewUser>) -> Result<Json<User>> {
    let password_hash = hash_password(req.password).await?;
    let email = req.email;
    let user_id = sqlx::query_scalar!(
        r#"insert into "user" (email, password_hash) values ($1, $2) returning user_id"#,
        email,
        password_hash
    )
    .fetch_one(&ctx.db)
    .await?
    .to_string();

    Ok(Json(User { user_id, email }))
}

async fn hash_password(password: String) -> Result<String> {
    // Argon2 hashing is designed to be computationally intensive,
    // so we need to do this on a blocking thread.
    tokio::task::spawn_blocking(move || -> Result<String> {
        let salt = SaltString::generate(rand::thread_rng());
        Ok(PasswordHash::generate(Argon2::default(), password, &salt)
            .map_err(|e| anyhow::anyhow!("failed to generate password hash: {}", e))?
            .to_string())
    })
    .await
    .context("panic in generating password hash")?
}

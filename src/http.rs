use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::async_trait;

use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash};
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

use crate::{config::Config, storage::UserStorage};

#[derive(Clone)]
pub(crate) struct ApiContext {
    config: Arc<Config>,
    storage: Arc<dyn UserStorage>,
}

pub use crate::error::Error;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub async fn serve(config: Config, db: PgPool) -> anyhow::Result<()> {
    let api_context = ApiContext {
        config: Arc::new(config),
        storage: Arc::new(PGUserStorage { db }),
    };
    let router = Router::new()
        .route("/user", post(create_user).get(get_user))
        .layer(TraceLayer::new_for_http())
        .with_state(api_context);
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8080));
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .context("error running HTTP server")
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct CreateUserResponse {
    user_id: String,
    email: String,
}

async fn create_user(
    ctx: State<ApiContext>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<CreateUserResponse>> {
    let res = ctx.storage.create_user(req).await?;
    Ok(Json(res))
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

#[derive(Deserialize)]
pub struct GetUserRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct GetUserResponse {
    user_id: String,
    email: String,
}

async fn get_user(
    ctx: State<ApiContext>,
    Json(req): Json<GetUserRequest>,
) -> Result<Json<GetUserResponse>> {
    let res = ctx.storage.get_user(req).await?;
    Ok(Json(res))
}

async fn verify_password(password: String, password_hash: String) -> Result<()> {
    tokio::task::spawn_blocking(move || -> Result<()> {
        let hash = PasswordHash::new(&password_hash)
            .map_err(|e| anyhow::anyhow!("invalid password hash: {}", e))?;

        hash.verify_password(&[&Argon2::default()], password)
            .map_err(|e| anyhow::anyhow!("failed to verify password hash: {}", e).into())
    })
    .await
    .context("panic in verifying password hash")?
}

pub struct PGUserStorage {
    db: PgPool,
}

#[async_trait]
impl UserStorage for PGUserStorage {
    async fn create_user(&self, req: CreateUserRequest) -> anyhow::Result<CreateUserResponse> {
        let password_hash = hash_password(req.password).await?;
        let email = req.email;
        let user_id = sqlx::query_scalar!(
            r#"insert into "user" (email, password_hash) values ($1, $2) returning user_id"#,
            email,
            password_hash
        )
        .fetch_one(&self.db)
        .await?
        .to_string();

        Ok(CreateUserResponse { user_id, email })
    }

    async fn get_user(&self, req: GetUserRequest) -> anyhow::Result<GetUserResponse> {
        let email = req.email;
        let user = sqlx::query!(
            r#"
                select user_id, email, password_hash 
                from "user" where email = $1
            "#,
            email,
        )
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("email is not found: {}", email))?;

        verify_password(req.password, user.password_hash).await?;

        let user_id = user.user_id.to_string();

        Ok(GetUserResponse { user_id, email })
    }
}

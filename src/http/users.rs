use crate::http::ApiContext;
use anyhow::bail;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};

/// A wrapper type for all requests/responses from these routes.
#[derive(serde::Serialize, serde::Deserialize)]
struct UserBody<T> {
    user: T,
}

struct NewUser {}
struct User {}

pub(crate) fn router() -> Router<ApiContext> {
    // By having each module responsible for setting up its own routing,
    // it makes the root module a lot cleaner.
    Router::new().route("/api/users", post(create_user))
}

async fn create_user(
    ctx: State<ApiContext>,
    Json(req): Json<UserBody<NewUser>>,
) -> Json<UserBody<User>> {
    unimplemented!()
}

use axum::async_trait;

use crate::http::{CreateUserRequest, CreateUserResponse, GetUserRequest, GetUserResponse};

#[async_trait]
pub trait UserStorage: Sync + Send {
    async fn create_user(&self, req: CreateUserRequest) -> anyhow::Result<CreateUserResponse>;
    async fn get_user(&self, req: GetUserRequest) -> anyhow::Result<GetUserResponse>;
}

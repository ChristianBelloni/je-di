use axum::{
    Router,
    http::{StatusCode, header::AUTHORIZATION},
    response::IntoResponse,
    routing::get,
};
use je_di::{axum::Dependency, axum_dependency, axum_world};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let db_connection = DBConnection;
    let router = Router::new()
        .route("/user", get(get_user))
        .with_state(db_connection);
    let listener = TcpListener::bind("[::]:3000").await.unwrap();

    axum::serve(listener, router).await.unwrap();
}

async fn get_user(
    Dependency(ValidatedUser(user_id)): Dependency<ValidatedUser>,
) -> impl IntoResponse {
    user_id.to_string()
}

struct AuthHeader(String);

#[derive(Clone)]
struct DBConnection;

impl DBConnection {
    pub async fn get_user_id(&self, token: String) -> Result<u64, String> {
        println!("validating token {token}");
        Ok(1)
    }
}

struct ValidatedUser(u64);

axum_world! {
    async fn from_world(parts: &Parts, _state: &DBConnection) -> Result<AuthHeader, StatusCode> {

        if let Some(header) = parts.headers.get(&AUTHORIZATION) {
            Ok(Self(
                header
                    .to_str()
                    .map_err(|_| StatusCode::UNAUTHORIZED)?
                    .to_string(),
            ))
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

axum_world! {
    async fn from_world(_p: &Parts, state: &DBConnection) -> Result<DBConnection, StatusCode> {
        Ok(state.clone())
    }
}

axum_dependency! {
    async fn from_dependency(_p: &Parts, state: &DBConnection, header: &AuthHeader) -> Result<ValidatedUser, StatusCode> {

         let user_id = match state.get_user_id(header.0.clone()).await {
             Ok(user_id) => user_id,
             Err(_) => return Err(StatusCode::UNAUTHORIZED),
         };
         Ok(ValidatedUser(user_id))
    }
}

use crate::async_dependency::FromAsyncWorld;
use async_trait::async_trait;
use axum::{
    extract::{FromRequest, FromRequestParts, Request},
    http::request::Parts,
    response::IntoResponse,
};

pub struct Dependency<T>(pub T);

impl<State, T> FromRequestParts<State> for Dependency<T>
where
    T: for<'a> FromAsyncWorld<World<'a> = (&'a Parts, &'a State)>,
    T::Error: IntoResponse,
    State: Sync,
{
    type Rejection = T::Error;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &State,
    ) -> Result<Self, Self::Rejection> {
        Ok(Dependency(T::from_world(&(parts, state)).await?))
    }
}

impl<State, T> FromRequest<State> for Dependency<T>
where
    T: for<'a> FromAsyncWorld<World<'a> = (Request, &'a State)>,
    T::Error: IntoResponse,
    State: Sync,
{
    type Rejection = T::Error;

    async fn from_request(
        req: axum::extract::Request,
        state: &State,
    ) -> Result<Self, Self::Rejection> {
        Ok(Dependency(T::from_world(&(req, state)).await?))
    }
}

#[cfg(test)]
#[allow(unused)]
mod example {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use axum::{
        Router,
        http::{HeaderMap, StatusCode},
        routing::post,
    };
    use tokio::net::TcpListener;

    use crate::{FromDependency, async_dependency::FromAsyncDependency};

    use super::*;

    #[derive(Clone)]
    struct AppState {
        db_connection: Arc<Mutex<Option<u32>>>,
    }

    #[derive(Clone, Debug)]
    struct AxumDep {
        headers: HeaderMap,
    }

    #[async_trait]
    impl FromAsyncWorld for AxumDep {
        type World<'a> = (&'a Parts, &'a AppState);

        type Error = StatusCode;

        async fn from_world<'a>(world: &'a Self::World<'a>) -> Result<Self, Self::Error> {
            Ok(Self {
                headers: world.0.headers.clone(),
            })
        }
    }

    #[derive(Clone, Debug)]
    struct AxumNestedDep {
        inner: AxumDep,
        connection: Arc<Mutex<Option<u32>>>,
    }

    #[async_trait]
    impl FromAsyncDependency for AxumNestedDep {
        type Error = StatusCode;

        type World<'a> = (&'a Parts, &'a AppState);

        type Dependency = AxumDep;

        async fn from_dependency(
            world: &Self::World<'_>,
            dependency: &Self::Dependency,
        ) -> Result<Self, Self::Error> {
            Ok(Self {
                inner: dependency.clone(),
                connection: world.1.db_connection.clone(),
            })
        }
    }

    #[tokio::test]
    async fn test_router() {
        let app = Router::new()
            .route("/my_route", post(handler))
            .with_state(AppState {
                db_connection: Default::default(),
            });

        let listener = TcpListener::bind("[::]:6000").await.unwrap();

        axum::serve(listener, app).await.unwrap();
    }

    async fn handler(
        Dependency(axum_dep): Dependency<AxumDep>,
        Dependency(nested): Dependency<AxumNestedDep>,
    ) {
        println!("{:?}", axum_dep);
        println!("{:?}", nested);
    }
}

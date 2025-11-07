//! # je-di Axum integration
//!
//! ## Usage
//!
//! ```ignore
//! use axum::{
//!     Router,
//!     http::{StatusCode, header::AUTHORIZATION},
//!     response::IntoResponse,
//!     routing::get,
//! };
//! use je_di::{axum::Dependency, axum_dependency, axum_world};
//! use tokio::net::TcpListener;
//!
//! #[tokio::main]
//! async fn main() {
//!     let db_connection = DBConnection;
//!     let router = Router::new()
//!         .route("/user", get(get_user))
//!         .with_state(db_connection);
//!     let listener = TcpListener::bind("[::]:3000").await.unwrap();
//!
//!     axum::serve(listener, router).await.unwrap();
//! }
//!
//! // Handler using our dependency
//! async fn get_user(
//!     Dependency(ValidatedUser(user_id)): Dependency<ValidatedUser>,
//! ) -> impl IntoResponse {
//!     user_id.to_string()
//! }
//!
//! // FromWorld dependency, reading from AUTHORIZATION header
//! struct AuthHeader(String);
//!
//! axum_world! {
//!     async fn from_world(parts: &Parts, _state: &DBConnection) -> Result<AuthHeader, StatusCode> {
//!         if let Some(header) = parts.headers.get(&AUTHORIZATION) {
//!             Ok(Self(
//!                 header
//!                     .to_str()
//!                     .map_err(|_| StatusCode::UNAUTHORIZED)?
//!                     .to_string(),
//!             ))
//!         } else {
//!             Err(StatusCode::UNAUTHORIZED)
//!         }
//!     }
//! }
//!
//! // FromWorld dependency, reading from State
//! #[derive(Clone)]
//! struct DBConnection;
//!
//! impl DBConnection {
//!     pub async fn get_user_id(&self, token: String) -> Result<u64, String> {
//!         println!("validating token {token}");
//!         Ok(1)
//!     }
//! }
//!
//! axum_world! {
//!     async fn from_world(_parts: &Parts, state: &DBConnection) -> Result<DBConnection, StatusCode> {
//!         Ok(state.clone())
//!     }
//! }
//!
//! // Leaf dependency, depends on DBConnection and AuthHeader
//! struct ValidatedUser(u64);
//!
//! // you can also manually implement FromAsyncDependency with the correct World type;
//!
//! impl FromAsyncDependency for ValidatedUser {
//!     type World<'a> = AxumRequestPartsWorld<'a>;
//!     type Error = StatusCode;
//!     type Dependency = AuthHeader;
//!
//!     async fn from_dependency(parts: &Parts, state: &DBConnection, header: &AuthHeader) -> Result<ValidatedUser, StatusCode> {
//!          let user_id = match state.get_user_id(header.0.clone()).await {
//!              Ok(user_id) => user_id,
//!              Err(_) => return Err(StatusCode::UNAUTHORIZED),
//!          };
//!          Ok(ValidatedUser(user_id))
//!     }
//! }
//!
//! // axum_dependency! {
//! //     async fn from_dependency(parts: &Parts, state: &DBConnection, header: &AuthHeader) -> Result<ValidatedUser, StatusCode> {
//! //          let user_id = match state.get_user_id(header.0.clone()).await {
//! //              Ok(user_id) => user_id,
//! //              Err(_) => return Err(StatusCode::UNAUTHORIZED),
//! //          };
//! //          Ok(ValidatedUser(user_id))
//! //     }
//! // }
//! ```

use crate::async_dependency::FromAsyncWorld;
use axum::{
    extract::{FromRequest, FromRequestParts, Request},
    http::request::Parts,
    response::IntoResponse,
};

/// Alias for a dependency that implements FromRequestParts via FromAsyncWorld
pub type AxumRequestPartsWorld<'a, State> = (&'a Parts, &'a State);

/// Alias for a dependency that implements FromRequest via FromAsyncWorld
pub type AxumRequestWorld<'a, State> = (Request, &'a State);

/// # Axum dependency extractor
///
/// implements [`FromRequest`]/[`FromRequestParts`]
/// where:
///
/// - `T` implements [`FromAsyncWorld`] where [`FromAsyncWorld::World`] = (&[`Parts`]/[`Request`], &State)
/// - `T::Error` implements [`IntoResponse`]
pub struct Dependency<T>(pub T);

impl<State, T> FromRequestParts<State> for Dependency<T>
where
    T: for<'a> FromAsyncWorld<World<'a> = AxumRequestPartsWorld<'a, State>>,
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
    T: for<'a> FromAsyncWorld<World<'a> = AxumRequestWorld<'a, State>>,
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

/// # Axum integration entry point
///
/// Define a FromAsyncWorld implementation that uses (Parts/Request, State) as World to enable seamless
/// integration with axum extractors
///
/// # Usage
/// ```ignore
/// use trait_id::axum::axum_world;
///
/// axum_world! { Type, RejectionType, StateType,
///     async fn from_world(
///         parts: &axum::http::request::Parts,
///         state: &StateType
///     ) -> Result<Self, Self::Error> {
///         // implementaiton returning Result<Self, RejectionType>
///     }
/// }
/// ```
#[macro_export]
macro_rules! axum_world {
    (
        async fn from_world(
            $parts:ident: &Parts,
            $state_ident:ident: &$state:ty
        ) -> Result<$ty:ty, $error:ty> { $($expr:tt)* }
    ) => {
        #[$crate::async_trait]
        impl $crate::async_dependency::FromAsyncWorld for $ty {
            type World<'a> = $crate::axum::AxumRequestPartsWorld<'a, $state>;
            type Error = $error;

            async fn from_world<'a>(world: &'a Self::World<'a>) -> Result<Self, Self::Error> {
                #[allow(unused)]
                let $parts = world.0;
                #[allow(unused)]
                let $state_ident = world.1;
                $($expr)*
            }
        }
    };

    (
        async fn from_world(
            $req:ident: Request,
            $state_ident:ident: &$state:ty
        ) -> Result<$ty:ty, $error:ty> { $($expr:tt)* }
    ) => {
        #[$crate::async_trait]
        impl $crate::async_dependency::FromAsyncWorld for $ty {
            type World<'a> = $crate::axum::AxumRequestWorld<'a, $state>;
            type Error = $error;

            async fn from_world<'a>(world: &'a Self::World<'a>) -> Result<Self, Self::Error> {
                #[allow(unused)]
                let $req = world.0;
                #[allow(unused)]
                let $state_ident = world.1;
                $($expr)*
            }
        }
    };
}
/// # Defines an axum aware dependency
///
/// Defines a FromAsyncDependency implementation that uses (Parts/Request, State) as World to
/// enable seamless integration with axum extractors
///
/// # Usage
/// ```ignore
/// use trait_id::axum::axum_dependency;
///
/// axum_dependency! { Type,
///     async fn from_world(
///         parts: &axum::http::request::Parts,
///         state: &StateType,
///         dependency: &DependencyType
///     ) -> Result<Self, RejectionType> {
///         // implementaiton returning Result<Self, RejectionType>
///     }
/// }
/// ```
#[macro_export]
macro_rules! axum_dependency {
    (
        async fn from_dependency(
            $req:ident: Request,
            $state_ident:ident: &$state:ty,
            $dependency_ident:ident: &$dependency:ty
        ) -> Result<$ty:ty, $error:ty> { $($expr:tt)* }
    ) => {
        #[$crate::async_trait]
        impl $crate::async_dependency::FromAsyncDependency for $ty {
            type Dependency = $dependency;
            type World<'a> = $crate::axum::AxumRequestWorld<'a, $state>;
            type Error = $error;

            async fn from_dependency(
                world: &Self::World<'_>,
                dependency: &Self::Dependency,
            ) -> Result<Self, Self::Error> {
                #[allow(unused)]
                let $req = world.0;
                #[allow(unused)]
                let $state_ident = world.1;
                let $dependency_ident = dependency;
                $($expr)*
            }
        }
    };

    (
        async fn from_dependency(
            $parts:ident: &Parts,
            $state_ident:ident: &$state:ty,
            $dependency_ident:ident: &$dependency:ty
        ) -> Result<$ty:ty, $error:ty> { $($expr:tt)* }
    ) => {
        #[$crate::async_trait]
        impl $crate::async_dependency::FromAsyncDependency for $ty {
            type Dependency = $dependency;
            type World<'a> = $crate::axum::AxumRequestPartsWorld<'a, $state>;
            type Error = $error;

            async fn from_dependency(
                world: &Self::World<'_>,
                dependency: &Self::Dependency,
            ) -> Result<Self, Self::Error> {
                #[allow(unused)]
                let $parts = world.0;
                #[allow(unused)]
                let $state_ident = world.1;
                let $dependency_ident = dependency;
                $($expr)*
            }
        }
    };
}

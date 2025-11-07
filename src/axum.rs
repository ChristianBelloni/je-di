use crate::async_dependency::FromAsyncWorld;
use axum::{
    extract::{FromRequest, FromRequestParts, Request},
    http::request::Parts,
    response::IntoResponse,
};

pub type AxumRequestPartsWorld<'a, State> = (&'a Parts, &'a State);
pub type AxumRequestWorld<'a, State> = (Request, &'a State);

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
        ) -> Result<$ty:ty, $error:ty> { $expr:expr }
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
                $expr
            }
        }
    };
}
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

    (
        $ty:ty, $error:ty,
        async fn from_dependency(
            $parts:ident: Request, 
            $state_ident:ident: &$state:ty, 
            $dependency_ident:ident: &$dependency:ty
        ) -> Result<Self, Self::Error> { $($expr:tt)* }
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
                let $parts = world.0;
                #[allow(unused)]
                let $state_ident = world.1;
                let $dependency_ident = dependency;
                $($expr)*
            }
        }
    };
}

//! je-di, compile time Hierarchical dependency injection framework
//!
//! # Basic usage
//! ```
//! // Define your 'World', in other words all the _always_ available structs and enums used by your
//! // application
//!
//! struct DBConnection(String);
//!
//! impl DBConnection {
//!    fn new(str: &str) -> std::io::Result<Self> { Ok(Self(str.to_string())) }
//! }
//!
//! #[derive(Clone)]
//! struct Client;
//! impl Client {
//!     pub fn new() -> Self { Self }
//!     pub async fn call_service(&self, url: &str) -> std::io::Result<u64> {
//!         Ok(1)
//!     }
//! }
//!
//! pub struct World {
//!     pub service_url: &'static str,
//!     pub http_client: Client,
//!     pub db_connection: DBConnection
//! }
//!
//! // Define your first level dependencies, these are all the dependencies that can be constructed
//! // with only a reference to World
//!
//! pub struct ServiceClient {
//!     url: &'static str,
//!     client: Client
//! }
//!
//! impl ServiceClient {
//!     async fn call_service(&self) -> std::io::Result<u64> {
//!         Ok(self.client.call_service(&self.url).await?)
//!     }
//! }
//!
//! // Then implement FromWorld/FromAsyncWorld for all these types
//!
//! use je_di::FromAsyncWorld;
//! use je_di::async_trait;
//!
//! #[async_trait]
//! impl FromAsyncWorld for ServiceClient {
//!     type World<'a> = World;
//!     type Error = std::io::Error;
//!
//!     async fn from_world<'a>(world: &'a Self::World<'a>) -> Result<Self, Self::Error> {
//!         Ok(Self { url: world.service_url, client: world.http_client.clone() })
//!     }
//! }
//!
//! // Then you can define the tree of dependencies starting from World and first level
//! // dependencies
//!
//! use je_di::{FromAsyncDependency, DIContainer};
//!
//! pub struct MeId(u64);
//!
//! #[async_trait]
//! impl FromAsyncDependency for MeId {
//!     type World<'a> = World;
//!     type Error = std::io::Error;
//!     type Dependency = ServiceClient;
//!
//!     async fn from_dependency(world: &Self::World<'_>, dependency: &Self::Dependency) -> Result<Self, Self::Error> {
//!         let this = dependency.call_service().await?;
//!         Ok(Self(this))
//!     }
//! }
//!
//! // Then using DIContainer extract all the necessary dependencies from your application
//!
//! async fn run_application(service_url: &'static str, connection_str: &str) -> std::io::Result<()> {
//!     let world = World {
//!         service_url,
//!         http_client: Client::new(),
//!         db_connection: DBConnection::new(connection_str)?
//!     };
//!
//!     let container = DIContainer::new(world);
//!
//!     let meid: MeId = container.extract_async().await?;
//!
//!     Ok(())
//! }
//! ```

#[cfg(feature = "axum")]
pub mod axum;

#[cfg(feature = "async")]
pub mod async_dependency;

#[cfg(feature = "async")]
pub use async_dependency::*;

#[cfg(feature = "async")]
pub use async_trait::async_trait;

/// # Entry point to je-di
///
/// Describes a struct that can be constructed from a given World
///
/// you can associate a type to a single World
///
/// # Usage
/// ```ignore
/// use je-di::FromWorld;
///
/// struct MyWorld(String);
///
/// struct MyDependency(String);
///
/// impl je_di::FromWorld for MyDependency {
///     type World<'a> = MyWorld;
///     type Error = MyError;
///
///     fn from_world<'a>(world: &'a Self::World<'a>) -> Result<Self, Self::Error> {
///         Ok(Self(world.0.clone()))
///     }
/// }
/// ```
pub trait FromWorld {
    type World<'a>;
    type Error;

    fn from_world(world: &Self::World<'_>) -> Result<Self, Self::Error>
    where
        Self: std::marker::Sized;
}

/// # Defines a dependency
///
/// Describes a struct that can be constructed from a world and a dependency that implements
/// FromWorld for the same World
///
/// # Usage
/// ```ignore
/// use je_di::FromDependency;
///
/// pub struct MyNestedDependency(OtherDependency);
///
/// impl FromDependency for MyNestedDependency {
///     type World<'a> = MyWorld;
///     type Error = MyError;
///     type Dependency = OtherDependency;
///
///     fn from_dependency(
///         _world: &Self::World,
///         dependency: &Self::Dependency
///     ) -> Result<Self, Self::Error> {
///         Ok(Self(dependency.clone()))
///     }
/// }
///
/// ```
pub trait FromDependency {
    type Error;
    type World<'a>;
    type Dependency: for<'a> FromWorld<World<'a> = Self::World<'a>, Error = Self::Error>;

    fn from_dependency(
        world: &Self::World<'_>,
        dependency: &Self::Dependency,
    ) -> Result<Self, Self::Error>
    where
        Self: std::marker::Sized;
}

impl<T> FromWorld for T
where
    T: FromDependency,
    T::Dependency: FromWorld,
{
    type Error = T::Error;
    type World<'a> = T::World<'a>;

    fn from_world(world: &Self::World<'_>) -> Result<Self, T::Error> {
        let dependency = <T::Dependency as FromWorld>::from_world(world)?;

        Self::from_dependency(world, &dependency)
    }
}

macro_rules! impl_tuple {
    ($first_n:tt:$first_name:ident, $($n:tt:$name:ident),+) => {
        impl<$first_name, $($name),*> FromWorld for ($first_name, $($name),+)
        where
            $first_name: FromWorld,
            $($name: for<'a> FromWorld<World<'a> = $first_name::World<'a>, Error = $first_name::Error>),*
        {
            type Error = $first_name::Error;
            type World<'a> = $first_name::World<'a>;

            fn from_world(world: &Self::World<'_>) -> Result<Self, Self::Error> {
                Ok((
                    $first_name::from_world(world)?,
                    $($name::from_world(world)?),+
                ))
            }
        }
    };
}

impl_tuple!(0:Dep0, 1:Dep1, 2:Dep2, 3:Dep3, 4:Dep4, 5:Dep5, 6:Dep6, 7:Dep7, 8:Dep8);
impl_tuple!(0:Dep0, 1:Dep1, 2:Dep2, 3:Dep3, 4:Dep4, 5:Dep5, 6:Dep6, 7:Dep7);
impl_tuple!(0:Dep0, 1:Dep1, 2:Dep2, 3:Dep3, 4:Dep4, 5:Dep5, 6:Dep6);
impl_tuple!(0:Dep0, 1:Dep1, 2:Dep2, 3:Dep3, 4:Dep4, 5:Dep5);
impl_tuple!(0:Dep0, 1:Dep1, 2:Dep2, 3:Dep3, 4:Dep4);
impl_tuple!(0:Dep0, 1:Dep1, 2:Dep2, 3:Dep3);
impl_tuple!(0:Dep0, 1:Dep1, 2:Dep2);
impl_tuple!(0:Dep0, 1:Dep1);

pub struct DIContainer<World> {
    world: World,
}

impl<World> DIContainer<World> {
    pub fn new(world: World) -> Self {
        Self { world }
    }

    pub fn extract<T: for<'a> FromWorld<World<'a> = World>>(
        &self,
    ) -> Result<T, <T as FromWorld>::Error> {
        T::from_world(&self.world)
    }
}

#[cfg(feature = "async")]
impl<World> DIContainer<World> {
    pub async fn extract_async<
        T: for<'a> crate::async_dependency::FromAsyncWorld<World<'a> = World>,
    >(
        &self,
    ) -> Result<T, <T as crate::async_dependency::FromAsyncWorld>::Error> {
        T::from_world(&self.world).await
    }
}

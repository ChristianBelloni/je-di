#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "axum")]
#[cfg_attr(docsrs, doc(cfg(feature = "axum")))]
pub mod axum;

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub mod async_dependency;

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub use async_dependency::*;

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
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
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
impl<World> DIContainer<World> {
    pub async fn extract_async<
        T: for<'a> crate::async_dependency::FromAsyncWorld<World<'a> = World>,
    >(
        &self,
    ) -> Result<T, <T as crate::async_dependency::FromAsyncWorld>::Error> {
        T::from_world(&self.world).await
    }
}

use async_trait::async_trait;

#[async_trait]
pub trait FromAsyncWorld: 'static {
    type World<'a>: Send + Sync;
    type Error: Send + Sync;

    async fn from_world<'a>(world: &'a Self::World<'a>) -> Result<Self, Self::Error>
    where
        Self: std::marker::Sized;
}

#[async_trait]
pub trait FromAsyncDependency: 'static {
    type Error: Send + Sync;
    type World<'a>: Send + Sync;
    type Dependency: for<'a> FromAsyncWorld<World<'a> = Self::World<'a>, Error = Self::Error> + Send;

    async fn from_dependency(
        world: &Self::World<'_>,
        dependency: &Self::Dependency,
    ) -> Result<Self, Self::Error>
    where
        Self: std::marker::Sized;
}

#[async_trait]
impl<T> FromAsyncWorld for T
where
    T: FromAsyncDependency,
    T::Dependency: FromAsyncWorld,
{
    type Error = T::Error;
    type World<'a> = T::World<'a>;

    async fn from_world<'a>(world: &'a Self::World<'a>) -> Result<Self, T::Error> {
        let dependency = <T::Dependency as FromAsyncWorld>::from_world(world).await?;

        Self::from_dependency(world, &dependency).await
    }
}

macro_rules! impl_tuple {
    ($first_n:tt:$first_name:ident, $($n:tt:$name:ident),+) => {
        #[async_trait]
        impl<$first_name, $($name),*> FromAsyncWorld for ($first_name, $($name),+)
        where
            $first_name: FromAsyncWorld + Send,
            $($name: Send + for<'a> FromAsyncWorld<World<'a> = $first_name::World<'a>, Error = $first_name::Error>),*
        {
            type Error = $first_name::Error;
            type World<'a> = $first_name::World<'a>;

            async fn from_world<'a>(world: &'a Self::World<'a>) -> Result<Self, Self::Error> {
                Ok((
                    $first_name::from_world(world).await?,
                    $($name::from_world(world).await?),+
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

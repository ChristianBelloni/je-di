#[cfg(feature = "axum")]
pub mod axum;

#[cfg(feature = "async")]
pub mod async_dependency;

pub trait FromWorld {
    type World<'a>;
    type Error;

    fn from_world(world: &Self::World<'_>) -> Result<Self, Self::Error>
    where
        Self: std::marker::Sized;
}

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

pub struct Container<World> {
    world: World,
}

impl<World> Container<World> {
    pub fn new(world: World) -> Self {
        Self { world }
    }

    pub fn extract<T: for<'a> FromWorld<World<'a> = World>>(
        &self,
    ) -> Result<T, <T as FromWorld>::Error> {
        T::from_world(&self.world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct Dep1;

    type MyWorld = String;
    type MyError = String;

    impl FromWorld for Dep1 {
        type World<'a> = MyWorld;
        type Error = MyError;

        fn from_world(_world: &Self::World<'_>) -> Result<Self, Self::Error> {
            Ok(Self)
        }
    }

    #[derive(Debug, Clone)]
    struct Dep2(Dep1);

    impl FromDependency for Dep2 {
        type Error = MyError;
        type World<'a> = MyWorld;

        type Dependency = Dep1;

        fn from_dependency(
            _world: &Self::World<'_>,
            dependency: &Self::Dependency,
        ) -> Result<Self, Self::Error> {
            Ok(Self(dependency.clone()))
        }
    }

    #[derive(Debug, Clone)]
    struct Dep3(Dep2);

    impl FromDependency for Dep3 {
        type Error = MyError;
        type World<'a> = MyWorld;

        type Dependency = Dep2;

        fn from_dependency(
            _world: &Self::World<'_>,
            dependency: &Self::Dependency,
        ) -> Result<Self, Self::Error> {
            Ok(Self(dependency.clone()))
        }
    }

    #[derive(Debug, Clone)]
    struct Dep4(Dep2, Dep3);

    impl FromDependency for Dep4 {
        type Error = MyError;
        type World<'a> = MyWorld;

        type Dependency = (Dep2, Dep3);

        fn from_dependency(
            _world: &Self::World<'_>,
            dependency: &Self::Dependency,
        ) -> Result<Self, Self::Error> {
            Ok(Self(dependency.0.clone(), dependency.1.clone()))
        }
    }

    #[derive(Debug, Clone)]
    struct Dep5(Dep1, Dep2, Dep3, Dep4);

    impl FromDependency for Dep5 {
        type Error = MyError;
        type World<'a> = MyWorld;

        type Dependency = (Dep1, Dep2, Dep3, Dep4);

        fn from_dependency(
            _world: &Self::World<'_>,
            dependency: &Self::Dependency,
        ) -> Result<Self, Self::Error> {
            Ok(Self(
                dependency.0.clone(),
                dependency.1.clone(),
                dependency.2.clone(),
                dependency.3.clone(),
            ))
        }
    }

    trait MyGenericDependency: Send + Sync + 'static {
        fn do_stuff(&self);
    }

    #[derive(Debug)]
    struct CustomImpl {
        pub inner: Dep5,
    }

    impl MyGenericDependency for CustomImpl {
        fn do_stuff(&self) {
            println!("custom impl {:?}", self.inner)
        }
    }

    impl FromDependency for Box<dyn MyGenericDependency> {
        type Error = MyError;
        type World<'a> = MyWorld;

        type Dependency = Dep5;

        fn from_dependency(
            _world: &Self::World<'_>,
            dependency: &Self::Dependency,
        ) -> Result<Self, Self::Error> {
            Ok(Box::new(CustomImpl {
                inner: dependency.clone(),
            }))
        }
    }

    #[test]
    fn basic_test() {
        let container = Container {
            world: "".to_string(),
        };

        let _d1: Dep1 = container.extract().unwrap();
        let _d2: Dep2 = container.extract().unwrap();
        let _d3: Dep3 = container.extract().unwrap();
        let _d4: Dep4 = container.extract().unwrap();
        let _d5: Dep5 = container.extract().unwrap();

        let d6: Box<dyn MyGenericDependency> = container.extract().unwrap();

        d6.do_stuff();
    }
}

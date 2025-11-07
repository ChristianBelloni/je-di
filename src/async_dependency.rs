//! je-di async dependency injection
//!
//! ```ignore
//! use je_di::{
//!     DIContainer,
//!     async_dependency::{FromAsyncDependency, FromAsyncWorld},
//!     async_trait,
//! };
//! use tokio::io::{AsyncBufReadExt, BufReader};
//!
//! struct World {
//!     username: String,
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let username = std::env::args()
//!         .nth(1)
//!         .expect("expected username")
//!         .to_string();
//!     let world = World { username };
//!     let container = DIContainer::new(world);
//!
//!     let mut stdin = BufReader::new(tokio::io::stdin());
//!     loop {
//!         let mut buf = String::new();
//!         stdin.read_line(&mut buf).await.unwrap();
//!         let buf = buf.trim();
//!
//!         let split = buf.split(" ").collect::<Vec<_>>();
//!         let operand = split[0];
//!
//!         match operand {
//!             "print" | "p" => {
//!                 let value = split[1];
//!                 handle_print(&container, value).await;
//!             }
//!             "loop" | "l" => {
//!                 let count = split[1].parse().unwrap();
//!                 let value = split[2];
//!                 handle_loop(&container, count, value).await;
//!             }
//!             "quit" | "q" => {
//!                 handle_quit(&container).await;
//!             }
//!             value => {
//!                 println!("unrecognized command {value}");
//!             }
//!         }
//!     }
//! }
//!
//! struct Looper {
//!     printer: Printer,
//! }
//!
//! #[derive(Clone)]
//! struct Printer {
//!     username: String,
//! }
//!
//! impl Printer {
//!     pub async fn print(&self, value: &str) {
//!         println!("[{}] {value}", self.username);
//!     }
//! }
//!
//! impl Looper {
//!     pub async fn loop_print(&self, count: usize, value: &str) {
//!         for _ in 0..count {
//!             self.printer.print(value).await;
//!         }
//!     }
//! }
//!
//! #[async_trait]
//! impl FromAsyncWorld for Printer {
//!     type World<'a> = World;
//!     type Error = String;
//!     async fn from_world<'a>(world: &'a Self::World<'a>) -> Result<Self, Self::Error> {
//!         Ok(Self {
//!             username: world.username.clone(),
//!         })
//!     }
//! }
//!
//! #[async_trait]
//! impl FromAsyncDependency for Looper {
//!     type Dependency = Printer;
//!     type World<'a> = World;
//!     type Error = String;
//!     async fn from_dependency(
//!         _: &Self::World<'_>,
//!         dependency: &Self::Dependency,
//!     ) -> Result<Self, Self::Error> {
//!         Ok(Self {
//!             printer: dependency.clone(),
//!         })
//!     }
//! }
//!
//! async fn handle_loop(container: &DIContainer<World>, count: usize, value: &str) {
//!     let looper: Looper = container.extract_async().await.unwrap();
//!
//!     looper.loop_print(count, value).await;
//! }
//!
//! async fn handle_print(container: &DIContainer<World>, value: &str) {
//!     let printer: Printer = container.extract_async().await.unwrap();
//!     printer.print(value).await;
//! }
//!
//! async fn handle_quit(container: &DIContainer<World>) {
//!     let printer: Printer = container.extract_async().await.unwrap();
//!     printer.print("quitting").await;
//!     std::process::exit(0);
//! }
//! ```

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
    type Dependency: for<'a> FromAsyncWorld<World<'a> = Self::World<'a>> + Send;

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
    T::Error: From<<T::Dependency as FromAsyncWorld>::Error>,
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

je-di, compile time Hierarchical dependency injection framework

# Basic usage
```
// Define your 'World', in other words all the _always_ available structs and enums used by your
// application

struct DBConnection(String);

impl DBConnection {
   fn new(str: &str) -> std::io::Result<Self> { Ok(Self(str.to_string())) }
}

#[derive(Clone)]
struct Client;
impl Client {
    pub fn new() -> Self { Self }
    pub async fn call_service(&self, url: &str) -> std::io::Result<u64> {
        Ok(1)
    }
}

pub struct World {
    pub service_url: &'static str,
    pub http_client: Client,
    pub db_connection: DBConnection
}

// Define your first level dependencies, these are all the dependencies that can be constructed
// with only a reference to World

pub struct ServiceClient {
    url: &'static str,
    client: Client
}

impl ServiceClient {
    async fn call_service(&self) -> std::io::Result<u64> {
        Ok(self.client.call_service(&self.url).await?)
    }
}

// Then implement FromWorld/FromAsyncWorld for all these types

use je_di::FromAsyncWorld;
use je_di::async_trait;

#[async_trait]
impl FromAsyncWorld for ServiceClient {
    type World<'a> = World;
    type Error = std::io::Error;

    async fn from_world<'a>(world: &'a Self::World<'a>) -> Result<Self, Self::Error> {
        Ok(Self { url: world.service_url, client: world.http_client.clone() })
    }
}

// Then you can define the tree of dependencies starting from World and first level
// dependencies

use je_di::{FromAsyncDependency, DIContainer};

pub struct MeId(u64);

#[async_trait]
impl FromAsyncDependency for MeId {
    type World<'a> = World;
    type Error = std::io::Error;
    type Dependency = ServiceClient;

    async fn from_dependency(world: &Self::World<'_>, dependency: &Self::Dependency) -> Result<Self, Self::Error> {
        let this = dependency.call_service().await?;
        Ok(Self(this))
    }
}

// Then using DIContainer extract all the necessary dependencies from your application

async fn run_application(service_url: &'static str, connection_str: &str) -> std::io::Result<()> {
    let world = World {
        service_url,
        http_client: Client::new(),
        db_connection: DBConnection::new(connection_str)?
    };

    let container = DIContainer::new(world);

    let meid: MeId = container.extract_async().await?;

    Ok(())
}
```

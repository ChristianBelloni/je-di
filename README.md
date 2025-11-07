je-di, compile time hierarchical dependency injection framework

# Basic usage
```rust
use je_di::{DIContainer, FromDependency, FromWorld, async_trait};
use std::io::{BufRead, BufReader};

struct World {
    username: String,
}

fn main() {
    let username = std::env::args()
        .nth(1)
        .expect("expected username")
        .to_string();
    let world = World { username };
    let container = DIContainer::new(world);

    let mut stdin = BufReader::new(std::io::stdin());
    loop {
        let mut buf = String::new();
        stdin.read_line(&mut buf).unwrap();
        let buf = buf.trim();

        let split = buf.split(" ").collect::<Vec<_>>();
        let operand = split[0];

        match operand {
            "print" | "p" => {
                let value = split[1];
                handle_print(&container, value);
            }
            "loop" | "l" => {
                let count = split[1].parse().unwrap();
                let value = split[2];
                handle_loop(&container, count, value);
            }
            "quit" | "q" => {
                handle_quit(&container);
            }
            value => {
                println!("unrecognized command {value}");
            }
        }
    }
}

struct Looper {
    printer: Printer,
}

#[derive(Clone)]
struct Printer {
    username: String,
}

impl Printer {
    pub fn print(&self, value: &str) {
        println!("[{}] {value}", self.username);
    }
}

impl Looper {
    pub fn loop_print(&self, count: usize, value: &str) {
        for _ in 0..count {
            self.printer.print(value);
        }
    }
}

#[async_trait]
impl FromWorld for Printer {
    type World<'a> = World;
    type Error = String;
    fn from_world<'a>(world: &'a Self::World<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            username: world.username.clone(),
        })
    }
}

#[async_trait]
impl FromDependency for Looper {
    type Dependency = Printer;
    type World<'a> = World;
    type Error = String;
    fn from_dependency(
        _: &Self::World<'_>,
        dependency: &Self::Dependency,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            printer: dependency.clone(),
        })
    }
}

fn handle_loop(container: &DIContainer<World>, count: usize, value: &str) {
    let looper: Looper = container.extract().unwrap();

    looper.loop_print(count, value);
}

fn handle_print(container: &DIContainer<World>, value: &str) {
    let printer: Printer = container.extract().unwrap();
    printer.print(value);
}

fn handle_quit(container: &DIContainer<World>) {
    let printer: Printer = container.extract().unwrap();
    printer.print("quitting");
    std::process::exit(0);
}
```

# Async integration
## Usage
```rust
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

# je-di Axum integration

## Usage

```rust
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

// Handler using our dependency
async fn get_user(
    Dependency(ValidatedUser(user_id)): Dependency<ValidatedUser>,
) -> impl IntoResponse {
    user_id.to_string()
}

// FromWorld dependency, reading from AUTHORIZATION header
struct AuthHeader(String);

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

// FromWorld dependency, reading from State
#[derive(Clone)]
struct DBConnection;

impl DBConnection {
    pub async fn get_user_id(&self, token: String) -> Result<u64, String> {
        println!("validating token {token}");
        Ok(1)
    }
}

axum_world! {
    async fn from_world(_parts: &Parts, state: &DBConnection) -> Result<DBConnection, StatusCode> {
        Ok(state.clone())
    }
}

// Leaf dependency, depends on DBConnection and AuthHeader
struct ValidatedUser(u64);

// you can also manually implement FromAsyncDependency with the correct World type;

impl FromAsyncDependency for ValidatedUser {
    type World<'a> = AxumRequestPartsWorld<'a>;
    type Error = StatusCode;
    type Dependency = AuthHeader;

    async fn from_dependency(parts: &Parts, state: &DBConnection, header: &AuthHeader) -> Result<ValidatedUser, StatusCode> {
         let user_id = match state.get_user_id(header.0.clone()).await {
             Ok(user_id) => user_id,
             Err(_) => return Err(StatusCode::UNAUTHORIZED),
         };
         Ok(ValidatedUser(user_id))
    }
}

// axum_dependency! {
//     async fn from_dependency(parts: &Parts, state: &DBConnection, header: &AuthHeader) -> Result<ValidatedUser, StatusCode> {
//          let user_id = match state.get_user_id(header.0.clone()).await {
//              Ok(user_id) => user_id,
//              Err(_) => return Err(StatusCode::UNAUTHORIZED),
//          };
//          Ok(ValidatedUser(user_id))
//     }
// }
```

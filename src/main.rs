use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use redis::{Client, aio::MultiplexedConnection};
use actix::prelude::*;
use log::{info, warn};

struct RedisActor {
    conn: MultiplexedConnection,
}

impl RedisActor {
    pub async fn new(redis_url: &'static str) -> Self {
        let client = Client::open(redis_url).unwrap();// not recommended
        info!(target: "redis_actor", "Starting Redis Connection");
        let (conn, call) = client.get_multiplexed_async_connection().await.unwrap();
        actix_rt::spawn(call);
        info!(target: "redis_actor", "Redis Connection ready");
        RedisActor { conn }
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<Option<String>, redis::RedisError>")]
struct InfoCommand;

impl Actor for RedisActor {
    type Context = Context<Self>;
}

impl Handler<InfoCommand> for RedisActor {
    type Result = ResponseFuture<Result<Option<String>, redis::RedisError>>;

    fn handle(&mut self, _msg: InfoCommand, _: &mut Self::Context) -> Self::Result {
        let mut con = self.conn.clone();
        let cmd = redis::cmd("INFO");
        let fut = async move {
            info!(target: "info_command", "Calling info command");
            cmd
                .query_async(&mut con)
                .await
        };
        Box::pin(fut)
    }
}

async fn info(redis: web::Data<Addr<RedisActor>>) -> impl Responder {
    info!(target: "info_request", "sending info command");
    let res = redis.send(InfoCommand).await.unwrap().unwrap().unwrap();
    info!(target: "info_request", "Got response {:?}", res);
    HttpResponse::Ok().body(res)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    warn!(target: "server", "Redis url is hadcoded");
    let actor = RedisActor::new("redis://127.0.0.1:6379").await;
    let addr = actor.start();
    HttpServer::new(move || {
        App::new()
            .data(addr.clone())
            .route("/", web::get().to(info))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
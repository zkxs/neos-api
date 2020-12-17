use std::borrow::Borrow;
use std::fmt;
use std::sync::Arc;

use futures::{FutureExt, StreamExt};
use tokio::sync::Mutex;
use warp::Filter;
use warp::http::{self, Response, StatusCode};
use warp::hyper::body::Bytes;

type Db = Arc<Mutex<Option<i64>>>;

#[tokio::main]
async fn main() {
    let counter_db: Db = Arc::new(Mutex::new(None));
    let init_timestamp_db: Db = Arc::new(Mutex::new(None));

    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = warp::get()
        .and(warp::path!("hello" / String))
        .map(|name| format!("Hello, {}!", name));

    // GET /hello => 200 OK with body "Hello!"
    let hello_fallback = warp::get()
        .and(warp::path("hello"))
        .map(|| "Hello!");

    // POST /initTime "100" => 200 OK with body "100"
    let init_time = warp::post()
        .and(warp::path("initTime"))
        // Only accept bodies smaller than 16kb...
        .and(warp::body::content_length_limit(1024 * 16))
        .and(with_db(init_timestamp_db.clone()))
        .and(warp::body::bytes())
        .and_then(init_time_handler);

    // POST /initTimeForce "100" => 200 OK with body "100"
    let init_time_force = warp::post()
        .and(warp::path("initTimeForce"))
        // Only accept bodies smaller than 16kb...
        .and(warp::body::content_length_limit(1024 * 16))
        .and(with_db(init_timestamp_db.clone()))
        .and(warp::body::bytes())
        .and_then(init_time_force_handler);

    // POST /initTimeReset => 200 OK
    let init_time_reset = warp::post()
        .and(warp::path("initTimeReset"))
        .and(warp::body::content_length_limit(0))
        .and(with_db(init_timestamp_db))
        .and_then(init_time_reset_handler);

    // GET /counter => 200 OK with body "Some(0)"
    let counter = warp::get()
        .and(warp::path("counter"))
        .and(with_db(counter_db).clone())
        .and_then(counter_handler);

    // WEBSOCKET /echo
    let websocket_test = warp::ws()
        .and(warp::path("echo"))
        .map(|ws: warp::ws::Ws| {
            // And then our closure will be called when it completes...
            ws.on_upgrade(|websocket| {
                // Just echo all messages back...
                let (tx, rx) = websocket.split();
                rx.forward(tx).map(|result| {
                    if let Err(e) = result {
                        eprintln!("websocket error: {:?}", e);
                    }
                })
            })
        });

    let routes = hello
        .or(hello_fallback)
        .or(init_time)
        .or(init_time_force)
        .or(init_time_reset)
        .or(counter)
        .or(websocket_test);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

fn with_db(db: Db) -> impl Filter<Extract=(Db, ), Error=std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}

async fn init_time_handler(db: Arc<Mutex<Option<i64>>>, bytes: Bytes) -> Result<http::Result<Response<String>>, warp::Rejection> {
    let init_time = match bytes_to_i64(bytes) {
        Ok(i64) => i64,
        Err(reply) => return Ok(reply),
    };
    let mut stored_init_time = db.lock().await;
    *stored_init_time = match *stored_init_time {
        Some(x) => Some(x),
        None => Some(init_time),
    };
    let init_time = stored_init_time.expect("stored_init_time should always be set at this point");
    Ok(Response::builder().status(StatusCode::OK).body(init_time.to_string()))
}

async fn init_time_reset_handler(db: Arc<Mutex<Option<i64>>>) -> Result<impl warp::Reply, warp::Rejection> {
    let mut stored_init_time = db.lock().await;
    *stored_init_time = None;
    Ok(StatusCode::OK)
}

async fn init_time_force_handler(db: Arc<Mutex<Option<i64>>>, bytes: Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    let init_time = match bytes_to_i64(bytes) {
        Ok(i64) => i64,
        Err(reply) => return Ok(reply),
    };
    let mut stored_init_time = db.lock().await;
    *stored_init_time = Some(init_time);
    Ok(Response::builder().status(StatusCode::OK).body(init_time.to_string()))
}

async fn counter_handler(db: Arc<Mutex<Option<i64>>>) -> Result<impl warp::Reply, warp::Rejection> {
    let mut counter = db.lock().await;
    *counter = match *counter {
        Some(x) => Some(x + 1),
        None => Some(0),
    };
    let x = *counter;
    Ok(option_to_string(x))
}

fn option_to_string<T: fmt::Display>(x: Option<T>) -> String {
    match x {
        Some(foo) => format!("Some({})", foo.to_string()),
        None => "None".to_string(),
    }
}

fn bytes_to_i64(bytes: Bytes) -> Result<i64, http::Result<Response<String>>> {
    let body = match std::str::from_utf8(bytes.borrow()) {
        Ok(body) => body,
        Err(utf8_error) => return Err(Response::builder().status(StatusCode::BAD_REQUEST).body(utf8_error.to_string())),
    };
    let value = match body.parse::<i64>() {
        Ok(i64) => i64,
        Err(parse_int_error) => return Err(Response::builder().status(StatusCode::BAD_REQUEST).body(parse_int_error.to_string())),
    };
    Ok(value)
}

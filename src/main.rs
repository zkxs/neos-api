#[macro_use]
extern crate lazy_static;

use std::{fmt, fs};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use app_dirs::{AppDataType, AppInfo};
use bytes::Buf as _;
use chrono::{DateTime, TimeZone, Utc};
use futures::{FutureExt, SinkExt, StreamExt};
use hyper::{Body, Client, Uri};
use hyper_tls::HttpsConnector;
use systemstat::{self, Platform};
use tokio::sync::{Mutex, MutexGuard};
use warp::Filter;
use warp::http::{self, Response, StatusCode};
use warp::hyper::body::Bytes;

use crate::dto::session_dto::Session;
use crate::dto::user_dto::{AbridgedUser, User};

mod dto;

type IntegerDb = Arc<Mutex<Option<i64>>>;
type SessionDb = Arc<Mutex<HashSet<String>>>;
type UserCacheDb = Arc<Mutex<HashMap<String, AbridgedUser>>>;

lazy_static! {
 static ref NEOS_SESSION_URI: Uri = "https://www.neosvr-api.com/api/sessions".parse().expect("Could not parse Neos session API URI");
 static ref CACHE_DIR_FILE_PATH: PathBuf = create_cache_file_path();
}

const NEOS_USER_URI: &str = "https://www.neosvr-api.com/api/users/";

// world IDs change on republish, so we'll just stick with name checking for now
const WORLD_NAME_PREFIXES: [&str; 5] = [
    "MTC",
    "Metaverse Training",
    "Neos Hub",
    "The Avatar Station",
    "Training"
];

const APP_INFO: AppInfo = AppInfo {
    name: env!("CARGO_PKG_NAME"),
    author: "runtime",
};

#[tokio::main]
async fn main() {
    println!("Initializing {} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let proxy_server_address: SocketAddr = ([127, 0, 0, 1], 3030).into();

    let counter_db: IntegerDb = Arc::new(Mutex::new(None));
    let init_timestamp_db: IntegerDb = Arc::new(Mutex::new(None));
    let session_db: SessionDb = Arc::new(Mutex::new(HashSet::new()));

    // attempt to load cache
    let loaded_cache = fs::read_to_string(CACHE_DIR_FILE_PATH.as_path())
        .map_err(|e| format!("{:?}", e))
        .and_then(|string| serde_json::from_str(&string).map_err(|e| format!("{:?}", e)));

    let default_cache = match loaded_cache {
        Ok(cache) => cache,
        Err(e) => {
            eprintln!("failed to load cache, defaulting to empty: {}", e);
            HashMap::new()
        }
    };
    let user_cache_db: UserCacheDb = Arc::new(Mutex::new(default_cache));

    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = warp::path!("hello" / String)
        .and(warp::get())
        .map(|name| format!("Hello, {}!", name));

    // GET /hello => 200 OK with body "Hello!"
    let hello_fallback = warp::path("hello")
        .and(warp::get())
        .map(|| "Hello!");

    // POST /initTime "100" => 200 OK with body "100"
    let init_time = warp::path("initTime")
        .and(warp::post())
        // Only accept bodies smaller than 16kb...
        .and(warp::body::content_length_limit(1024 * 16))
        .and(with_db(init_timestamp_db.clone()))
        .and(warp::body::bytes())
        .and_then(init_time_handler);

    // POST /initTimeForce "100" => 200 OK with body "100"
    let init_time_force = warp::path("initTimeForce")
        .and(warp::post())
        // Only accept bodies smaller than 16kb...
        .and(warp::body::content_length_limit(1024 * 16))
        .and(with_db(init_timestamp_db.clone()))
        .and(warp::body::bytes())
        .and_then(init_time_force_handler);

    // POST /initTimeReset => 200 OK
    let init_time_reset = warp::path("initTimeReset")
        .and(warp::post())
        .and(warp::body::content_length_limit(0))
        .and(with_db(init_timestamp_db.clone()))
        .and_then(init_time_reset_handler);

    // GET /initTimePeek => 200 OK with body "Some(100)"
    let init_time_peek = warp::path("initTimePeek")
        .and(warp::get())
        .and(with_db(init_timestamp_db))
        .and_then(init_time_peek_handler);

    // GET /counter => 200 OK with body "Some(0)"
    let counter = warp::path("counter")
        .and(warp::get())
        .and(with_db(counter_db).clone())
        .and_then(counter_handler);

    // GET /systemstat => 200 OK with body containing many system stats
    let systemstat = warp::path("systemstat")
        .and(warp::get())
        .map(get_system_stat);

    let sessionlist = warp::path("sessionlist")
        .and(warp::get())
        .and(with_db(session_db))
        .and(with_db(user_cache_db))
        .and_then(sessionlist_handler);

    // WEBSOCKET /echo
    let echo = warp::path("echo")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            // And then our closure will be called when it completes...
            ws.on_upgrade(|websocket| {
                // Just echo all messages back...
                let (tx, rx) = websocket.split();
                rx.forward(tx).map(|result| {
                    if let Err(e) = result {
                        eprintln!("websocket echo error: {:?}", e);
                    }
                })
            })
        });

    // WEBSOCKET /wshello
    let ws_hello = warp::path("wshello")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            println!("incoming wshello connection");
            // And then our closure will be called when it completes...
            ws.on_upgrade(wshello_handler)
        });

    let routes = hello
        .or(hello_fallback)
        .or(init_time)
        .or(init_time_force)
        .or(init_time_reset)
        .or(init_time_peek)
        .or(systemstat)
        .or(sessionlist)
        .or(counter)
        .or(ws_hello)
        .or(echo);

    println!("Starting web server...");
    warp::serve(routes)
        .run(proxy_server_address)
        .await;
}

fn with_db<T: Clone + Send>(db: T) -> impl Filter<Extract=(T, ), Error=std::convert::Infallible> + Clone {
    warp::any().map(move || db.clone())
}

async fn sessionlist_handler(db: SessionDb, user_cache: UserCacheDb) -> Result<impl warp::Reply, warp::Rejection> {
    let uri = (*NEOS_SESSION_URI).clone();
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    let response: Response<Body> = match client.get(uri).await {
        Ok(r) => r,
        Err(e) => return Ok(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(format!("Error reading neos session api response: {:?}", e)))
    };
    let sessions = match deserialize_session(response).await {
        Ok(s) => s,
        Err(e) => return Ok(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(format!("Error parsing neos session api response: {:?}", e)))
    };

    let sessions = sessions.into_iter()
        .filter(
            |s| (WORLD_NAME_PREFIXES.iter().any(|prefix| s.name.starts_with(prefix)))
                && s.is_valid
                && !s.has_ended
                && s.active_users > 0
                && host_present(s)
        )
        .collect::<Vec<Session>>();

    let new_set = sessions.iter()
        .map(|s| s.session_id.clone())
        .collect::<HashSet<String>>();

    let mut session_db_mutex = db.lock().await;
    let notification_needed = new_set.difference(&*session_db_mutex).next().is_some();
    *session_db_mutex = new_set;
    drop(session_db_mutex);


    let mut user_cache_mutex = user_cache.lock().await;
    let current_time = Utc::now();
    let mut session_list_string = Vec::with_capacity(sessions.len());
    for session in sessions.into_iter() {
        let session_start_time = session.session_begin_time.parse::<DateTime<Utc>>().unwrap_or(Utc.timestamp_millis(0));
        let uptime = current_time.signed_duration_since(session_start_time);
        let user_data_string = match session.host_user_id {
            Some(user_id) => {
                match lookup_user_cached(&mut user_cache_mutex, user_id).await {
                    Ok(user) => {
                        let registration_date = format!(" {}", format_user_registration_date(&user));
                        let is_patron = (if user.is_patron { " patron" } else { "" }).to_string();
                        format!("{}{}", registration_date, is_patron)
                    }
                    Err(err) => format!(" {}", err)
                }
            }
            None => String::new(),
        };

        // return a tuple so that we can sort this by an i64 later
        let new_element = (
            session_start_time.timestamp_millis(),
            format!("{} ({}) ({}/{}) {}:{:02}{}", session.host_username, session.name, session.active_users, session.joined_users, uptime.num_seconds() / 60, uptime.num_seconds() % 60, user_data_string)
        );
        session_list_string.push(new_element);
    }
    drop(user_cache_mutex);

    // unstable sort is fine as long as no sessions were started in the same millisecond
    session_list_string.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    let session_list_string = session_list_string
        .into_iter()
        .map(|(_, string)| string)
        .collect::<Vec<String>>()
        .join("\n");

    let prefix_string = if notification_needed {
        "N"
    } else {
        "X"
    };
    let session_list_string = format!("{}{}", prefix_string, session_list_string);
    Ok(Response::builder().status(StatusCode::OK).body(session_list_string))
}

fn host_present(session: &Session) -> bool {
    let users = &session.session_users;
    if session.host_user_id.is_some() {
        users.into_iter().any(|u| u.is_present && u.user_id == session.host_user_id)
    } else {
        users.into_iter().any(|u| u.is_present && u.username == session.host_username)
    }
}

async fn deserialize_session(response: Response<Body>) -> Result<Vec<Session>, String> {
    let body = hyper::body::aggregate(response).await
        .map_err(|e| format!("error aggregating session response body: {:?}", e))?;
    serde_json::from_reader(body.reader())
        .map_err(|e| format!("error parsing session response body: {:?}", e))
}

// wshello handler
async fn wshello_handler(websocket: warp::ws::WebSocket) {
    println!("/wshello: handler called");

    let (mut tx, mut rx) = websocket.split();

    println!("/wshello: connected");

    while let Some(result) = rx.next().await {
        let message = match result {
            Ok(message) => {
                println!("/wshello: received {:?}", message);
                message
            }
            Err(e) => {
                eprintln!("/wshello: message error: {:?}", e);
                break;
            }
        };
        let message = match message.to_str() {
            Ok(str) => str,
            Err(e) => {
                eprintln!("/wshello: error converting message to string: {:?}", e);
                continue;
            }
        };
        let message = format!("Hello, {}!", message);
        match tx.send(warp::ws::Message::text(message)).await {
            Ok(e) => println!("/wshello: sending message: {:?}", e),
            Err(e) => eprintln!("/wshello: error sending message: {:?}", e),
        };
    }
    println!("/wshello: disconnected");
}

// normal init_time route handler
async fn init_time_handler(db: Arc<Mutex<Option<i64>>>, bytes: Bytes) -> Result<http::Result<Response<String>>, warp::Rejection> {
    let init_time = match bytes_to_i64(bytes) {
        Ok(i64) => i64,
        Err(reply) => return Ok(reply),
    };
    let mut stored_init_time_mutex = db.lock().await;
    *stored_init_time_mutex = match *stored_init_time_mutex {
        Some(x) => Some(x),
        None => Some(init_time),
    };
    let init_time = (*stored_init_time_mutex).expect("stored_init_time should always be set at this point");
    Ok(Response::builder().status(StatusCode::OK).body(init_time.to_string()))
}

// handler to reset the internal init_time state
async fn init_time_reset_handler(db: Arc<Mutex<Option<i64>>>) -> Result<impl warp::Reply, warp::Rejection> {
    let mut stored_init_time_mutex = db.lock().await;
    *stored_init_time_mutex = None;
    Ok(StatusCode::OK)
}

// handler to force the internal init_time state to a given number
async fn init_time_force_handler(db: Arc<Mutex<Option<i64>>>, bytes: Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    let init_time = match bytes_to_i64(bytes) {
        Ok(i64) => i64,
        Err(reply) => return Ok(reply),
    };
    let mut stored_init_time_mutex = db.lock().await;
    *stored_init_time_mutex = Some(init_time);
    Ok(Response::builder().status(StatusCode::OK).body(init_time.to_string()))
}

// handler to peek the init_time without modification
async fn init_time_peek_handler(db: Arc<Mutex<Option<i64>>>) -> Result<impl warp::Reply, warp::Rejection> {
    let stored_init_time_mutex = db.lock().await;
    Ok(Response::builder().status(StatusCode::OK).body(option_to_string(*stored_init_time_mutex)))
}

// handler it increment a nullable counter
async fn counter_handler(db: Arc<Mutex<Option<i64>>>) -> Result<impl warp::Reply, warp::Rejection> {
    let mut counter_mutex = db.lock().await;
    *counter_mutex = match *counter_mutex {
        Some(x) => Some(x + 1),
        None => Some(0),
    };
    let x = *counter_mutex;
    Ok(option_to_string(x))
}

// convert an option to a pretty string
fn option_to_string<T: fmt::Display>(x: Option<T>) -> String {
    match x {
        Some(foo) => format!("Some({})", foo.to_string()),
        None => "None".to_string(),
    }
}

// bytes --> utf8 string --> i64
fn bytes_to_i64(bytes: Bytes) -> Result<i64, http::Result<Response<String>>> {
    let value = match std::str::from_utf8(bytes.borrow()) {
        Ok(str) => str,
        Err(utf8_error) => return Err(Response::builder().status(StatusCode::BAD_REQUEST).body(utf8_error.to_string())),
    };
    let value = match value.parse::<i64>() {
        Ok(i64) => i64,
        Err(parse_int_error) => return Err(Response::builder().status(StatusCode::BAD_REQUEST).body(parse_int_error.to_string())),
    };
    Ok(value)
}

fn get_system_stat() -> String {
    let sys = systemstat::System::new();

    let mounts = match sys.mounts() {
        Ok(mounts) => {
            let mut string = String::from("Mounts:");
            for mount in mounts.iter() {
                string.push_str(
                    format!(
                        "\n    {} --- {} ---> {} (available {} of {})",
                        mount.fs_mounted_from, mount.fs_type, mount.fs_mounted_on, mount.avail, mount.total
                    ).as_str()
                );
            }
            string
        }
        Err(x) => format!("Mounts: error: {}", x)
    };

    let block_device_statistics = match sys.block_device_statistics() {
        Ok(stats) => {
            let mut string = String::from("Block devices:");
            for blkstats in stats.values() {
                string.push_str(format!("\n    {}: {:?}", blkstats.name, blkstats).as_str());
            }
            string
        }
        Err(x) => format!("Block devices: error: {}", x.to_string())
    };

    let networks = match sys.networks() {
        Ok(netifs) => {
            let mut string = String::from("Networks:");
            for netif in netifs.values() {
                string.push_str(format!("\n    {} ({:?})", netif.name, netif.addrs).as_str());
            }
            string
        }
        Err(x) => format!("Networks: error: {}", x)
    };

    let interfaces = match sys.networks() {
        Ok(netifs) => {
            let mut string = String::from("Interfaces:");
            for netif in netifs.values() {
                string.push_str(format!("\n    {} statistics: ({:?})", netif.name, sys.network_stats(&netif.name)).as_str());
            }
            string
        }
        Err(x) => format!("Interfaces: error: {}", x)
    };

    let battery = match sys.battery_life() {
        Ok(battery) =>
            format!("Battery: {}%, {}h{}m remaining",
                    battery.remaining_capacity * 100.0,
                    battery.remaining_time.as_secs() / 3600,
                    battery.remaining_time.as_secs() % 60),
        Err(x) => format!("Battery: error: {}", x)
    };

    let power = match sys.on_ac_power() {
        Ok(power) => format!(", AC power: {}", power),
        Err(x) => format!(", AC power: error: {}", x)
    };

    let memory = match sys.memory() {
        Ok(mem) => format!("Memory: {} used / {} ({} bytes) total ({:?})", systemstat::saturating_sub_bytes(mem.total, mem.free), mem.total, mem.total.as_u64(), mem.platform_memory),
        Err(x) => format!("Memory: error: {}", x)
    };

    let load = match sys.load_average() {
        Ok(loadavg) => format!("Load average: {} {} {}", loadavg.one, loadavg.five, loadavg.fifteen),
        Err(x) => format!("Load average: error: {}", x)
    };

    let uptime = match sys.uptime() {
        Ok(uptime) => format!("Uptime: {:?}", uptime),
        Err(x) => format!("Uptime: error: {}", x)
    };

    let boot_time = match sys.boot_time() {
        Ok(boot_time) => format!("Boot time: {}", boot_time),
        Err(x) => format!("Boot time: error: {}", x)
    };

    // match sys.cpu_load_aggregate() {
    //     Ok(cpu)=> {
    //         println!("\nMeasuring CPU load...");
    //         thread::sleep(Duration::from_secs(1));
    //         let cpu = cpu.done().unwrap();
    //         println!("CPU load: {}% user, {}% nice, {}% system, {}% intr, {}% idle ",
    //                  cpu.user * 100.0, cpu.nice * 100.0, cpu.system * 100.0, cpu.interrupt * 100.0, cpu.idle * 100.0);
    //     },
    //     Err(x) => println!("\nCPU load: error: {}", x)
    // }

    let cpu_temp = match sys.cpu_temp() {
        Ok(cpu_temp) => format!("CPU temp: {}", cpu_temp),
        Err(x) => format!("CPU temp: {}", x)
    };

    let socket_stats = match sys.socket_stats() {
        Ok(stats) => format!("System socket statistics: {:?}", stats),
        Err(x) => format!("System socket statistics: error: {}", x.to_string())
    };

    format!(
        "{}\n{}\n{}\n{}\n{}{}\n{}\n{}\n{}\n{}\n{}\n{}",
        mounts,
        block_device_statistics,
        networks,
        interfaces,
        battery,
        power,
        memory,
        load,
        uptime,
        boot_time,
        cpu_temp,
        socket_stats
    )
}

fn format_user_registration_date(user: &AbridgedUser) -> String {
    match lookup_user_registration_date(user) {
        Ok(registration_date) => registration_date.date().naive_local().to_string(),
        Err(error) => error
    }
}

fn lookup_user_registration_date(user: &AbridgedUser) -> Result<DateTime<Utc>, String> {
    user.registration_date.parse::<DateTime<Utc>>()
        .map_err(|e| format!("parse error: {:?}", e))
}

async fn lookup_user_cached(cache_mutex: &mut MutexGuard<'_, HashMap<String, AbridgedUser>>, user_id: String) -> Result<AbridgedUser, String> {
    match cache_mutex.get(&user_id) {
        Some(user) => {
            Ok(user.clone())
        }
        None => {
            println!("caching new user {}", user_id);
            let user: AbridgedUser = lookup_user(&user_id).await?.into();
            cache_mutex.insert(user_id, user.clone());
            Ok(user)
        }
    }
}

async fn lookup_user(user_id: &str) -> Result<User, String> {
    let uri: Uri = format!("{}{}", NEOS_USER_URI, user_id).parse()
        .map_err(|e| format!("Could not parse Neos user API URI: {:?}", e))?;
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    let response: Response<Body> = client.get(uri).await
        .map_err(|e| format!("Could not read Neos user API response body: {:?}", e))?;
    deserialize_user(response).await
}

async fn deserialize_user(response: Response<Body>) -> Result<User, String> {
    let body = hyper::body::aggregate(response).await
        .map_err(|e| format!("error aggregating user response body: {:?}", e))?;
    serde_json::from_reader(body.reader())
        .map_err(|e| format!("error parsing user response body: {:?}", e))
}

fn create_cache_file_path<'a>() -> PathBuf {
    let config_dir_path = app_dirs::get_app_root(AppDataType::UserConfig, &APP_INFO).expect("unable to locate configuration directory");
    fs::create_dir_all(config_dir_path.as_path()).expect("failed to create configuration directory");
    config_dir_path.join("cache.json")
}

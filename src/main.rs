use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{stdin, Read},
    sync::{Arc, RwLock},
};
use tokio::runtime::Runtime;
use warp::{http, Filter};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AirplaneStatus {
    identifier: String,
    altitude: i32,
}

#[derive(Clone)]
struct Database {
    airplane_list: Arc<RwLock<HashMap<String, i32>>>,
}

impl Database {
    fn new() -> Self {
        Database {
            airplane_list: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

fn main() {
    let database = Database::new();

    println!("Spawning Warp Endpoint");
    let rt = Runtime::new().unwrap();
    rt.spawn(async move {
        launch_warp(database).await;
    });

    println!("Doing Other Stuff");

    stdin().read(&mut [0u8]).unwrap();
    println!("Exiting Main");
}

async fn launch_warp(database: Database) {
    println!("Launching Warp Endpoint");

    let database_filter = warp::any().map(move || database.clone());

    // POST localhost:3030/ {"identifier": "ZZ123", "altitude": 15000}
    let add_airplane = warp::post()
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .and(database_filter.clone())
        .and_then(update_airplane_database);

    // GET localhost:3030/database
    let get_database = warp::get()
        .and(warp::path::path("database"))
        .and(warp::path::end())
        .and(database_filter.clone())
        .and_then(get_airplane_database);

    // POST localhost:3030/raw
    let post_raw_bytes = warp::post()
        .and(warp::path::path("raw"))
        .and(warp::path::end())
        .and(warp::body::bytes())
        .and_then(process_raw_bytes);

    let routes = add_airplane.or(get_database).or(post_raw_bytes);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn update_airplane_database(
    airplane: AirplaneStatus,
    database: Database,
) -> Result<impl warp::Reply, warp::Rejection> {
    println!("Add airplane to database");
    println!("{:?}", airplane);

    database
        .airplane_list
        .write()
        .unwrap()
        .insert(airplane.identifier, airplane.altitude);
    Ok(warp::reply::with_status(
        "Added Airplane to Database",
        http::StatusCode::CREATED,
    ))
}

async fn get_airplane_database(database: Database) -> Result<impl warp::Reply, warp::Rejection> {
    println!("Sending database to client");

    let hashmap_out = database.airplane_list.read().unwrap();
    Ok(warp::reply::json(&*hashmap_out))
}

async fn process_raw_bytes(buf: Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    println!("raw bytes = {:?}", buf);
    let v: AirplaneStatus = serde_json::from_slice(&buf).unwrap();
    println!("{:?}", v);
    Ok(warp::reply())
}

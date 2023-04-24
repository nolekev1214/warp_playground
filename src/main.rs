mod database;

use bytes::Bytes;
use database::Database;
use std::io::{stdin, Read};
use std::net::SocketAddrV4;
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, mpsc::Sender, oneshot};
use warp::{http, Filter};

#[derive(Debug)]
struct InternalChannelError;
impl warp::reject::Reject for InternalChannelError {}

const DATABASE_MESSAGE_QUEUE_SIZE: usize = 1000;
const SERVER_ADDRESS: &str = "127.0.0.1:3030";

fn main() {
    let (tx, rx) = mpsc::channel(DATABASE_MESSAGE_QUEUE_SIZE);
    Database::start_message_processor(rx);

    println!("Spawning Warp Endpoint");
    let rt = Runtime::new().unwrap();
    rt.spawn(async move {
        launch_warp(tx).await;
    });

    println!("Doing Other Stuff");

    stdin().read_exact(&mut [0_u8; 1]).unwrap();
    println!("Exiting Main");
}

async fn launch_warp(tx: Sender<database::Request>) {
    println!("Launching Warp Endpoint");

    let send_filter = warp::any().map(move || tx.clone());

    // POST localhost:3030/ {"identifier": "ZZ123", "altitude": 15000}
    let add_airplane = warp::post()
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .and(send_filter.clone())
        .and_then(update_airplane_database);

    // GET localhost:3030/database
    let get_database = warp::get()
        .and(warp::path::path("database"))
        .and(warp::path::end())
        .and(send_filter.clone())
        .and_then(get_airplane_database);

    // GET localhost:3030/database/ {"identifier": "ZZ123"}
    let get_airplane = warp::get()
        .and(warp::path::path("database"))
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .and(send_filter.clone())
        .and_then(get_airplane);

    // POST localhost:3030/raw
    let post_raw_bytes = warp::post()
        .and(warp::path::path("raw"))
        .and(warp::path::end())
        .and(warp::body::bytes())
        .and_then(process_raw_bytes);

    let routes = add_airplane
        .or(get_airplane)
        .or(get_database)
        .or(post_raw_bytes);

    let server_address: SocketAddrV4 = SERVER_ADDRESS.parse().unwrap();
    warp::serve(routes).run(server_address).await;
}

async fn update_airplane_database(
    airplane: database::AirplaneStatus,
    send: Sender<database::Request>,
) -> Result<impl warp::Reply, warp::Rejection> {
    println!("Add airplane to database");
    println!("{:?}", airplane);

    let msg = database::Request::Add(airplane);
    send.send(msg).await.unwrap();

    Ok(warp::reply::with_status(
        "Added Airplane to Database",
        http::StatusCode::CREATED,
    ))
}

async fn get_airplane_database(
    send: Sender<database::Request>,
) -> Result<impl warp::Reply, warp::Rejection> {
    println!("Sending database to client");

    let (tx, rx) = oneshot::channel();

    let msg = database::Request::GetDB(tx);

    if (send.send(msg).await).is_err() {
        Err(warp::reject::custom(InternalChannelError))
    } else {
        let response = rx.await.unwrap();
        Ok(warp::reply::json(&response))
    }
}

async fn get_airplane(
    airplane: database::AirplaneId,
    send: Sender<database::Request>,
) -> Result<impl warp::Reply, warp::Rejection> {
    println!("Sending single airplane to client");

    let (tx, rx) = oneshot::channel();

    let msg = database::Request::GetAirplane((airplane, tx));

    if (send.send(msg).await).is_err() {
        Err(warp::reject::custom(InternalChannelError))
    } else {
        let response = rx.await.unwrap();
        Ok(warp::reply::json(&response))
    }
}

async fn process_raw_bytes(buf: Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    println!("raw bytes = {:?}", buf);
    let v: database::AirplaneStatus = serde_json::from_slice(&buf).unwrap();
    println!("{:?}", v);
    Ok(warp::reply())
}

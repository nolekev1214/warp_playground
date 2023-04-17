mod database;

use bytes::Bytes;
use crossbeam::channel::{self, Receiver, Sender};
use std::future;
use std::io::{stdin, Read};
use std::task::Poll;
use tokio::runtime::Runtime;
use uuid::Uuid;
use warp::{http, Filter};

fn main() {
    let (send_to_db, recv_from_main) = channel::unbounded();
    let (send_to_main, recv_from_db) = channel::unbounded();
    database::Database::start_message_processor(recv_from_main, send_to_main);

    println!("Spawning Warp Endpoint");
    let rt = Runtime::new().unwrap();
    rt.spawn(async move {
        launch_warp(recv_from_db, send_to_db).await;
    });

    println!("Doing Other Stuff");

    stdin().read(&mut [0u8]).unwrap();
    println!("Exiting Main");
}

async fn launch_warp(rx: Receiver<database::Response>, tx: Sender<database::Request>) {
    println!("Launching Warp Endpoint");

    let send_filter = warp::any().map(move || tx.clone());
    let recv_filter = warp::any().map(move || rx.clone());

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
        .and(recv_filter.clone())
        .and_then(get_airplane_database);

    let get_airplane = warp::get()
        .and(warp::path::path("database"))
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .and(send_filter.clone())
        .and(recv_filter.clone())
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

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn update_airplane_database(
    airplane: database::AirplaneStatus,
    send: Sender<database::Request>,
) -> Result<impl warp::Reply, warp::Rejection> {
    println!("Add airplane to database");
    println!("{:?}", airplane);

    let uuid = Uuid::new_v4();
    let msg = database::Request {
        uuid: uuid,
        request: database::RequestMessage::Add(airplane),
    };

    send.send(msg).unwrap();

    Ok(warp::reply::with_status(
        "Added Airplane to Database",
        http::StatusCode::CREATED,
    ))
}

async fn get_airplane_database(
    send: Sender<database::Request>,
    recv: Receiver<database::Response>,
) -> Result<impl warp::Reply, warp::Rejection> {
    println!("Sending database to client");

    let uuid = Uuid::new_v4();
    let msg = database::Request {
        uuid: uuid,
        request: database::RequestMessage::GetDB,
    };

    send.send(msg).unwrap();

    let response = future::poll_fn(|_cx| get_response_from_db(uuid, recv.clone())).await;

    Ok(warp::reply::json(&response.response))
}

async fn get_airplane(
    airplane: database::AirplaneId,
    send: Sender<database::Request>,
    recv: Receiver<database::Response>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let uuid = Uuid::new_v4();
    let msg = database::Request {
        uuid: uuid,
        request: database::RequestMessage::GetAirplane(airplane),
    };

    send.send(msg).unwrap();

    let response = future::poll_fn(|_cx| get_response_from_db(uuid, recv.clone())).await;

    Ok(warp::reply::json(&response.response))
}

fn get_response_from_db(
    uuid: uuid::Uuid,
    recv: Receiver<database::Response>,
) -> Poll<database::Response> {
    match recv.iter().find(|response| response.uuid == uuid) {
        Some(response) => Poll::Ready(response),
        None => Poll::Pending,
    }
}

async fn process_raw_bytes(buf: Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    println!("raw bytes = {:?}", buf);
    let v: database::AirplaneStatus = serde_json::from_slice(&buf).unwrap();
    println!("{:?}", v);
    Ok(warp::reply())
}

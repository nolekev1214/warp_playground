#![deny(warnings)]
use std::{
    collections::HashMap,
    io::{stdin, Read},
};
use tokio::runtime::Runtime;
use warp::{http::Response, Filter};

fn main() {
    println!("Spawning Warp Endpoint");
    let rt = Runtime::new().unwrap();
    rt.spawn(async move {
        launch_warp().await;
    });

    println!("Doing Other Stuff");

    stdin().read(&mut [0u8]).unwrap();
    println!("Exiting Main");
}

async fn launch_warp() {
    println!("Launching Warp Endpoint");

    // GET /?name=<var>
    let get_params = warp::path::end()
        .and(warp::query::<HashMap<String, String>>())
        .map(|p: HashMap<String, String>| match p.get("name") {
            Some(name) => Response::builder().body(format!("Hello, {}", name)),
            None => Response::builder().body(String::from("No \"name\" param in query.")),
        });

    // GET /ids/
    let return_json = warp::path("ids").map(|| {
        let our_ids = vec![1, 2, 6, 4];
        warp::reply::json(&our_ids)
    });

    let routes = get_params.or(return_json);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot::Sender;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AirplaneStatus {
    identifier: String,
    altitude: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AirplaneId {
    identifier: String,
}

#[derive(Debug)]
pub enum Request {
    Add(AirplaneStatus),
    GetAirplane((AirplaneId, Sender<Response>)),
    GetDB(Sender<Response>),
}

#[derive(Debug, Serialize)]
pub enum Response {
    Airplane(Option<AirplaneStatus>),
    Database(HashMap<String, i32>),
}

pub struct Database {
    airplane_list: Arc<RwLock<HashMap<String, i32>>>,
    command_channel: Receiver<Request>,
    thread_pool: Runtime,
}

impl Database {
    pub fn start_message_processor(rx: Receiver<Request>) {
        let mut db = Database {
            airplane_list: Arc::new(RwLock::new(HashMap::new())),
            command_channel: rx,
            thread_pool: Runtime::new().unwrap(),
        };
        thread::spawn(move || db.process_messages());
    }

    fn process_messages(&mut self) {
        loop {
            match self.command_channel.blocking_recv() {
                Some(command) => {
                    let airplane_list = self.airplane_list.clone();
                    self.thread_pool.spawn(async {
                        Database::process_message(airplane_list, command).await;
                    })
                }
                None => return,
            };
        }
    }

    async fn process_message(airplane_list: Arc<RwLock<HashMap<String, i32>>>, command: Request) {
        match command {
            Request::Add(airplane) => Database::add_airplane(airplane_list, airplane).await,
            Request::GetAirplane((airplane, tx)) => {
                Database::get_airplane(airplane_list, airplane, tx).await
            }
            Request::GetDB(tx) => Database::get_database(airplane_list, tx).await,
        }
    }

    async fn add_airplane(
        airplane_list: Arc<RwLock<HashMap<String, i32>>>,
        airplane: AirplaneStatus,
    ) {
        airplane_list
            .write()
            .await
            .insert(airplane.identifier, airplane.altitude);
    }

    async fn get_database(
        airplane_list: Arc<RwLock<HashMap<String, i32>>>,
        response_channel: Sender<Response>,
    ) {
        let response = Response::Database(airplane_list.read().await.clone());
        response_channel.send(response).unwrap();
    }

    async fn get_airplane(
        airplane_list: Arc<RwLock<HashMap<String, i32>>>,
        airplane_id: AirplaneId,
        response_channel: Sender<Response>,
    ) {
        let response =
            if let Some(altitude) = airplane_list.read().await.get(&airplane_id.identifier) {
                let airplane_status = AirplaneStatus {
                    altitude: *altitude,
                    identifier: airplane_id.identifier,
                };
                Response::Airplane(Some(airplane_status))
            } else {
                Response::Airplane(None)
            };
        response_channel.send(response).unwrap()
    }
}

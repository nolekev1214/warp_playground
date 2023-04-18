use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot::Sender;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AirplaneStatus {
    identifier: String,
    altitude: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AirplaneId {
    identifier: String,
}

pub enum Request {
    Add(AirplaneStatus),
    GetAirplane((AirplaneId, Sender<Response>)),
    GetDB(Sender<Response>),
}

#[derive(Serialize)]
pub enum Response {
    Airplane(Option<AirplaneStatus>),
    Database(HashMap<String, i32>),
}

pub struct Database {
    airplane_list: HashMap<String, i32>,
    command_channel: Receiver<Request>,
}

impl Database {
    pub fn start_message_processor(rx: Receiver<Request>) {
        let mut db = Database {
            airplane_list: HashMap::new(),
            command_channel: rx,
        };
        thread::spawn(move || db.process_messages());
    }

    fn process_messages(self: &mut Self) {
        loop {
            match self.command_channel.blocking_recv().unwrap() {
                Request::Add(airplane) => {
                    self.airplane_list
                        .insert(airplane.identifier, airplane.altitude);
                }
                Request::GetDB(tx) => {
                    let response = Response::Database(self.airplane_list.clone());
                    tx.send(response);
                }
                Request::GetAirplane((airplane, tx)) => {
                    let response =
                        if let Some(altitude) = self.airplane_list.get(&airplane.identifier) {
                            let airplane_status = AirplaneStatus {
                                altitude: *altitude,
                                identifier: airplane.identifier,
                            };
                            Response::Airplane(Some(airplane_status))
                        } else {
                            Response::Airplane(None)
                        };
                    tx.send(response);
                }
            };
        }
    }
}

use crossbeam::channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AirplaneStatus {
    identifier: String,
    altitude: i32,
}

pub struct Request {
    pub uuid: uuid::Uuid,
    pub request: RequestMessage,
}

pub enum RequestMessage {
    Add(AirplaneStatus),
    GetDB,
}

pub struct Response {
    pub uuid: uuid::Uuid,
    pub response: ResponseMessage,
}

#[derive(Serialize)]
pub enum ResponseMessage {
    Airplane(AirplaneStatus),
    Database(HashMap<String, i32>),
}

pub struct Database {
    airplane_list: HashMap<String, i32>,
}

impl Database {
    pub fn start_message_processor(rx: Receiver<Request>, tx: Sender<Response>) {
        let mut db = Database {
            airplane_list: HashMap::new(),
        };
        thread::spawn(move || db.process_messages(rx, tx));
    }

    fn process_messages(self: &mut Self, rx: Receiver<Request>, tx: Sender<Response>) {
        loop {
            let message = rx.recv().unwrap();
            match message.request {
                RequestMessage::Add(airplane) => {
                    self.airplane_list
                        .insert(airplane.identifier, airplane.altitude);
                }
                RequestMessage::GetDB => {
                    let response = ResponseMessage::Database(self.airplane_list.clone());
                    let msg = Response {
                        uuid: message.uuid,
                        response: response,
                    };
                    tx.send(msg).unwrap();
                }
            };
        }
    }
}

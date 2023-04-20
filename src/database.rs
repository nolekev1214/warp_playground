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

    fn process_messages(&mut self) {
        loop {
            let command = self.command_channel.blocking_recv();
            if command.is_none(){
                return;
            }
            match command.unwrap() {
                Request::Add(airplane) => self.add_airplane(airplane),
                Request::GetDB(tx) => self.get_database(tx),
                Request::GetAirplane((airplane, tx)) => {
                    self.get_airplane(airplane, tx);
                }
            };
        }
    }

    fn add_airplane(&mut self, airplane: AirplaneStatus) {
        self.airplane_list
            .insert(airplane.identifier, airplane.altitude);
    }

    fn get_database(&self, response_channel: Sender<Response>) {
        let response = Response::Database(self.airplane_list.clone());
        response_channel.send(response).unwrap();
    }

    fn get_airplane(&self, airplane_id: AirplaneId, response_channel: Sender<Response>) {
        let response = if let Some(altitude) = self.airplane_list.get(&airplane_id.identifier) {
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

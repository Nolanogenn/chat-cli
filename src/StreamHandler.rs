use std::net::{TcpListener, TcpStream};
use std::{
    sync::mpsc, net::SocketAddr,
    time::Duration,
    io::Write
};
use crate::EventHandlers::*;

pub struct Client{
    connected: bool,
    stream: Option<TcpStream>,
    tx: mpsc::Sender<Event>,
}

impl Client {
    pub fn new(
        tx: mpsc::Sender<Event>
        ) -> Self {
        Self{
            connected: false,
            stream: None,
            tx: tx
        }
    }
    pub fn accept_connection(& mut self, address: SocketAddr) -> std::io::Result<()>{
        match TcpStream::connect_timeout(
            &address, Duration::from_secs(5)){
            Ok(stream) => {
                self.connected = true;
                self.stream = Some(stream);
            }
            Err(_) => {
                self.tx.send(Event::ConnectionKo(address)).unwrap();
            }
        }
        Ok(())
    }
    pub fn connect_to(& mut self, address: SocketAddr) -> std::io::Result<()>{
        match TcpStream::connect_timeout(
            &address, Duration::from_secs(5)){
            Ok(stream) => {
                self.connected = true;
                let local_addr = stream.local_addr().unwrap().ip();
                self.stream = Some(stream);
                self.tx.send(Event::ConnectionOk(address,local_addr)).unwrap();
            }
            Err(_) => {
                self.tx.send(Event::ConnectionKo(address)).unwrap();
            }
        }
        Ok(())
    }
    pub fn write(& mut self, msg: String){
        match & mut self.stream{
            Some(s) => s.write(msg.as_bytes()),
            _ => panic!("something went wrong"),
        };
    }
}

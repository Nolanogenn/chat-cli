use crossterm::event::{KeyEvent};
use std::{io::BufReader, prelude::*};
use std::{
    sync::mpsc,
    net::TcpStream,
    net::TcpListener,
    net::SocketAddr,
};

pub enum Event<'a>{
    Input(KeyEvent),
    ConnectionOk(SocketAddr,&'a TcpStream),
    ConnectionKo(SocketAddr),
    TcpMessageIn(String),
    Error(String)
}

pub fn handle_listener_events(tx:mpsc::Sender<Event>){
    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let buf_reader = BufReader::new(&stream);
        for line in buf_reader.lines(){
            match line {
                Ok(msg) => tx.send(Event::TcpMessageIn(msg)),
                Err(e) => tx.send(Event::Error(format!("{}", e)))
            }
        };
    };
}

pub fn handle_input_events(tx: mpsc::Sender<Event>){
    loop {
        match crossterm::event::read().unwrap(){
            crossterm::event::Event::Key(key_event) => tx.send(Event::Input(key_event)).unwrap(),
            _ => {}
        }
    }
}


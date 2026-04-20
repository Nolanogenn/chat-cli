use crossterm::event::{KeyEvent};
use std::{
    sync::mpsc,
    net::TcpStream,
    net::TcpListener,
    net::SocketAddr
};

pub enum Event<'a>{
    Input(KeyEvent),
    ConnectionOk(SocketAddr,&'a TcpStream),
    ConnectionKo(SocketAddr),
    TcpMessage(String)
}

pub fn handle_listener_events(listener: &TcpListener, tx:mpsc::Sender<Event>){
    for stream in listener.incoming() {
        todo!()
    }
}

pub fn handle_input_events(tx: mpsc::Sender<Event>){
    loop {
        match crossterm::event::read().unwrap(){
            crossterm::event::Event::Key(key_event) => tx.send(Event::Input(key_event)).unwrap(),
            _ => {}
        }
    }
}


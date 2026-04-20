use crossterm::event::{KeyEvent};
use std::{
    sync::mpsc,
    net::TcpStream,
    net::SocketAddr
};

pub enum Event{
    Input(KeyEvent),
    ConnectionOk(SocketAddr,TcpStream),
    ConnectionKo(SocketAddr),
}

pub fn handle_input_events(tx: mpsc::Sender<Event>){
    loop {
        match crossterm::event::read().unwrap(){
            crossterm::event::Event::Key(key_event) => tx.send(Event::Input(key_event)).unwrap(),
            _ => {}
        }
    }
}


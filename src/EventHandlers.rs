use crossterm::event::{KeyEvent};
use std::{
    sync::mpsc,
    net::TcpStream
};

pub enum Event{
    Input(KeyEvent),
    ConnectionOk(String,TcpStream),
    ConnectionKo(String),
}

pub fn handle_input_events(tx: mpsc::Sender<Event>){
    loop {
        match crossterm::event::read().unwrap(){
            crossterm::event::Event::Key(key_event) => tx.send(Event::Input(key_event)).unwrap(),
            _ => {}
        }
    }
}


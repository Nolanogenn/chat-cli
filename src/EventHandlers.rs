use crossterm::event::{KeyEvent};
use std::{
    sync::mpsc,
};

pub enum Event{
    Input(KeyEvent),
}

pub fn handle_input_events(tx: mpsc::Sender<Event>){
    loop {
        match crossterm::event::read().unwrap(){
            crossterm::event::Event::Key(key_event) => tx.send(Event::Input(key_event)).unwrap(),
            _ => {}
        }
    }
}


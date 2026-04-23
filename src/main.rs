use std::net::TcpListener;
use ratatui::widgets::{ListState};
use std::{
    io,
    thread,
    sync::mpsc,
};

mod ChatApp;
mod EventHandlers;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let (event_tx, event_rx) = mpsc::channel::<EventHandlers::Event>();
    let tx_to_input_events = event_tx.clone();
    thread::spawn(move || {
        EventHandlers::handle_input_events(tx_to_input_events);
    });
    let list_state = ListState::default().with_selected(Some(0));
    let mut items: Vec<String> = Vec::with_capacity(99);
    let tx_to_listener_events = event_tx.clone();
    thread::spawn(move || {
        EventHandlers::handle_listener_events(tx_to_listener_events);
    });
    items.push("new connection".to_string());
    let mut app = ChatApp::App::new(
        list_state,
        items,
        event_rx,
        event_tx
        );
    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result
}



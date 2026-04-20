use std::net::{TcpListener, TcpStream};
use crossterm::event::{KeyCode};
use ratatui::layout::{Constraint, Layout,Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, List, ListItem, Paragraph, ListState};
use ratatui::{DefaultTerminal, Frame};
use std::{
    io,
    sync::mpsc,
    thread,
    time::Duration,
    net::SocketAddr
};
use crate::EventHandlers::*;

pub struct App {
    exit: bool,
    msg: String,
    input: String,
    connected: bool,
    character_index: usize,
    input_mode: InputMode,
    list_state: ListState,
    items: Vec<String>,
    listener: TcpListener,
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>
}

enum InputMode {
    Error,
    List,
    Connecting,
    Waiting,
    WaitingForResponse
}

impl App {
    pub fn new(
        list_state: ListState,
        items: Vec<String>,
        listener: TcpListener,
        rx: mpsc::Receiver<Event>,
        tx: mpsc::Sender<Event>
        ) -> Self {
        Self {
            exit: false,
            connected: false,
            msg: String::new(),
            input: String::new(),
            input_mode: InputMode::List,
            character_index: 0,
            list_state: list_state,
            items: items,
            listener: listener,
            rx: rx,
            tx: tx
        }
    }
    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }
    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }
    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }
    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }
    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    const fn reset_cursor(&mut self) {
        self.character_index = 0;
    }
    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> io::Result<()> {
        match self.input_mode {
            InputMode::WaitingForResponse => match key_event.code {
                KeyCode::Esc => {
                    self.input_mode = InputMode::Connecting
                },
                _ => {}
            }
            InputMode::Error => match key_event.code{
                KeyCode::Esc => {
                    self.input_mode = InputMode::Connecting
                },
                _ => {}
            },
            InputMode::List => match key_event.code{
                KeyCode::Down => self.list_state.select_next(),
                KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Esc => { self.exit = true; },
                KeyCode::Enter => {
                    match self.list_state.selected() {
                        Some(0) => { self.input_mode = InputMode::Connecting},
                        _ => {}
                    }
                }
                _ => {}
            },
            InputMode::Connecting => {
                match key_event.code{
                    KeyCode::Esc => { self.input_mode = InputMode::List},
                    KeyCode::Enter => {
                        self.input_mode = InputMode::Waiting;
                        let addr_str = format!("{}:7878", self.input);
                        let addr: SocketAddr = addr_str
                            .parse()
                            .expect("unable to pase");
                        self.try_connection(addr);
                        },
                    KeyCode::Char(to_insert) => self.enter_char(to_insert),
                    KeyCode::Backspace => self.delete_char(),
                    _ => {}
                }
            },
            InputMode::Waiting => {
                match key_event.code {
                KeyCode::Esc => self.input_mode = InputMode::Connecting,
                _ => {}
                }
            }
        }
        Ok(())
    }
    fn try_connection(& self, addr: SocketAddr){
        let tx_to_connection_events = self.tx.clone();
        thread::spawn(move || {
            match TcpStream::connect_timeout(
                &addr, Duration::from_secs(10)
                ){
                Ok(stream) => {
                    tx_to_connection_events.send(
                        Event::ConnectionOk(
                            addr, stream)).unwrap();
                }
                Err(_) => {
                    tx_to_connection_events.send(
                        Event::ConnectionKo(
                            addr)).unwrap();
                }
            }
        });
    }
    fn handle_connection_ok(&mut self) -> io::Result<()>{
        self.input_mode = InputMode::WaitingForResponse;
        Ok(())
    }
    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        ) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render(frame))?;
            match self.rx.recv().unwrap(){
                    Event::Input(key_event) => self.handle_key_event(key_event)?,
                    Event::ConnectionOk(addr, stream) => self.handle_connection_ok()?,
                    Event::ConnectionKo(addr) => todo!(),
                    _ => todo!()
                }
        }
        Ok(())
    }
    fn render(&mut self, frame: &mut Frame) {
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
        ]);
        let [_top,first,_second] = frame.area().layout(&layout);
        let [help_area, input_area, messages_area] = frame.area().layout(&layout);
        let (msg, style) = match self.input_mode {
            InputMode::WaitingForResponse => (
                vec!["Connessione stabilita. In attesa di una risposta...".bold()],
                Style::default()
                ),
            InputMode::List => (
                vec!["Possible options".bold()],
                Style::default()
                ),
            InputMode::Connecting => (
                vec!["Write the ip you are trying to contact".bold()],
                Style::default()
                ),
            InputMode::Error => (
                vec!["ERRORE".bold()],
                Style::default()
                ),
            InputMode::Waiting => (
                vec!["In attesa...".bold()],
                Style::default()
                )
        };
        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);
        match self.input_mode {
            InputMode::List => self.render_list(frame, first),
            InputMode::Connecting => {
                let input = Paragraph::new(self.input.as_str())
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::bordered().title("IP"));
                frame.render_widget(input, input_area)
            },
            InputMode::Error => {
                let error_message = Paragraph::new(self.msg.as_str())
                    .style(Style::default().fg(Color::Red))
                    .block(Block::bordered().title("ERROR"));
                frame.render_widget(error_message, messages_area)
            },
            InputMode::Waiting => {},
            InputMode::WaitingForResponse => {}
        }
    }
    
    pub fn render_list(&mut self, frame: &mut Frame, area:Rect){
        let list = List::new(self.items.clone())
            .style(Color::White)
            .highlight_style(Modifier::REVERSED)
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }
}

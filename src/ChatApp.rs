use regex::Regex;
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
    net::{SocketAddr, IpAddr}
};
use crate::EventHandlers::*;
use crate::StreamHandler::*;

pub struct App{
    exit: bool,
    msg: String,
    messages: Vec<String>,
    input: String,
    connected: bool,
    character_index: usize,
    input_mode: InputMode,
    list_state: ListState,
    items: Vec<String>,
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
    client: Client,
    username: String,
    local_addr: Option<IpAddr>
}

enum InputMode {
    Error,
    List,
    Connecting,
    Connected,
    Waiting,
    WaitingForResponse
}

impl App {
    pub fn new(
        list_state: ListState,
        items: Vec<String>,
        rx: mpsc::Receiver<Event>,
        tx: mpsc::Sender<Event>,
        client_tx: mpsc::Sender<Event>,
        username: String,
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
            rx: rx,
            tx: tx,
            client: Client::new(client_tx),
            username: username,
            local_addr: None,
            messages: Vec::new(),
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
    fn submit_message(&mut self){
        self.messages.push(self.input.clone());
        self.input.clear();
        self.reset_cursor();
        self.write_msg("MSG".to_string(), self.input.clone());
    }
    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> io::Result<()> {
        match self.input_mode {
            InputMode::Connected => match key_event.code {
                KeyCode::Esc => {
                    self.list_state.select_first();
                    self.input_mode = InputMode::List
                },
                KeyCode::Char(to_insert) => self.enter_char(to_insert),
                KeyCode::Backspace => self.delete_char(),
                KeyCode::Left => self.move_cursor_left(),
                KeyCode::Right => self.move_cursor_right(),
                KeyCode::Enter => {
                    self.submit_message();
                }
                _ => {}
            }
            InputMode::WaitingForResponse => match key_event.code {
                KeyCode::Esc => {
                    self.write_msg(
                        "CLOSECONN".to_string(),
                        self.local_addr.unwrap().to_string());
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
                        Some(0) => {
                            self.input_mode = InputMode::Connecting
                        },
                        Some(n) => {
                            self.accept_conn(n);
                        },
                        _ => {}
                    }
                }
                _ => {}
            },
            InputMode::Connecting => {
                match key_event.code{
                    KeyCode::Esc => { 
                    self.write_msg(
                        "CLOSECONN".to_string(),
                        self.local_addr.unwrap().to_string());
                    self.input_mode = InputMode::List
                },
                    KeyCode::Enter => {
                        self.input_mode = InputMode::Waiting;
                        let addr_str = format!("{}:7878", self.input);
                        let addr: SocketAddr = addr_str
                            .parse()
                            .expect("unable to parse");
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
    fn write_msg(& mut self, command: String, msg: String){
        self.client.write(
            format!("<{}> <{}> {}\r\n", command, self.username, msg)
            .to_string()
        );
    }
    fn accept_conn(& mut self, n: usize)-> io::Result<()>{
        let user: Vec<&str> = self.items.get(n).unwrap().split(' ').collect();
        let addr = format!("{}:7878", user[1]);
        self.client.accept_connection(
            addr.parse().expect(
                &format!("unable to parse: {}", addr)
                ));
        self.input = "".to_string();
        self.input_mode = InputMode::Connected;
        Ok(())
    }
    fn try_connection(& mut self, addr: SocketAddr){
        self.client.connect_to(addr);
    }
    fn remove_conn(& mut self, username: String, addr: String){
        if let Some(pos) = self.items.iter().position(
            |x| x.contains(&username) && x.contains(&addr)
            ) {
            self.items.remove(pos);
        }
    }
    fn handle_connection_accepted(&mut self, local_addr: IpAddr) -> io::Result<()>{
        self.input = "".to_string();
        self.input_mode = InputMode::Connected;
        Ok(())
    }
    fn handle_connection_ok(&mut self, local_addr: IpAddr) -> io::Result<()>{
        self.input_mode = InputMode::WaitingForResponse;
        self.local_addr = Some(local_addr);
        self.write_msg("TRYCONN".to_string(), self.local_addr.unwrap().to_string()); 
        Ok(())
    }
    fn handle_connection_ko(&mut self) -> io::Result<()>{
        self.msg = "Impossibile stabilire una connessione".to_string();
        self.input_mode = InputMode::Error;
        Ok(())
    }
    fn handle_message_in(&mut self, msg: &str) -> io::Result<()>{
        let re = Regex::new(r"^<([^>]+)> <([^>]+)> (.*)$").unwrap(); 
        if let Some(caps)  = re.captures(msg) {
            let command = &caps[1];
            let username = &caps[2];
            let msg = &caps[3];
            match command {
                "TRYCONN" =>{
                    match self.input_mode{
                        InputMode::WaitingForResponse => {
                            self.input = "".to_string();
                            self.input_mode = InputMode::Connected
                        },
                        _ => {self.items.push(
                            format!(
                                "{} {}",
                                username.to_string(),
                                msg.to_string()))
                        }
                    }
                },
                "CLOSECONN" => {
                    self.remove_conn(username.to_string(), msg.to_string())
                },
                "MSG" => {
                    self.receive_msg(username.to_string(), msg.to_string())
                },
                _ => todo!("{}", command),
            }
        }
        Ok(())
    }
    fn receive_msg(&mut self, username: String, msg: String) {
        self.messages.push(format!("{}: {}", username, msg));
    }
    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        ) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render(frame))?;
            match self.rx.recv().unwrap(){
                    Event::Input(key_event) => self.handle_key_event(key_event)?,
                    Event::ConnectionAccepted(addr, local_addr) => self.handle_connection_accepted(local_addr)?,
                    Event::ConnectionOk(addr,local_addr) => self.handle_connection_ok(local_addr)?,
                    Event::ConnectionKo(addr) => self.handle_connection_ko()?,
                    Event::TcpMessageIn(msg) => self.handle_message_in(& msg)?,
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
            InputMode::Connected => (
                vec!["chat".bold()],
                Style::default(),
                ),
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
            InputMode::Connected => {
                let input = Paragraph::new(self.input.as_str())
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::bordered().title("Input"));
                frame.render_widget(input, input_area);
            }
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

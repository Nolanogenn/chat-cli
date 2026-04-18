use crossterm::event::{KeyCode};
use ratatui::layout::{Constraint, Layout,Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, List, ListItem, Paragraph, ListState};
use ratatui::{DefaultTerminal, Frame};
use std::{
    io,
    sync::mpsc,
};
use crate::EventHandlers::*;

pub struct App {
    exit: bool,
    input: String,
    character_index: usize,
    input_mode: InputMode,
    list_state: ListState,
    items: Vec<String>
}

enum InputMode {
    List,
    Connecting
}

impl App {
    pub fn new(list_state: ListState, items: Vec<String>) -> Self {
        Self {
            exit: false,
            input: String::new(),
            input_mode: InputMode::List,
            character_index: 0,
            list_state: list_state,
            items: items
        }
    }
    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> io::Result<()> {
        match self.input_mode {
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
                    KeyCode::Esc => { self.exit = true};
                }
            }
        }
        Ok(())
    }
    pub fn run(&mut self, terminal: &mut DefaultTerminal, rx: mpsc::Receiver<Event>) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render(frame))?;
            match rx.recv().unwrap(){
                    Event::Input(key_event) => self.handle_key_event(key_event)?,
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
        let [help_area, input_area, _messages_area] = frame.area().layout(&layout);
        let (msg, style) = match self.input_mode {
            InputMode::List => (
                vec!["Possible options".bold()],
                Style::default()
                ),
            InputMode::Connecting => (
                vec!["Write the ip you are trying to contact".bold()],
                Style::default()
                ),
        };
        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);
        match self.input_mode {
            InputMode::List => self.render_list(frame, first),
            InputMode::Connecting => {}
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

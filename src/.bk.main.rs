use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::{
    io::{BufReader, prelude::*},
    net::{TcpListener, TcpStream},
    time::Duration,
    io,
    thread,
    sync::mpsc
};

fn prompt(text: &str) -> String {
    print!("{}", text);
    std::io::stdout().flush().expect("something went awrong");
    let mut response = String::new();
    std::io::stdin()
        .read_line(&mut response)
        .expect("failed to get input");
    response.trim_end().to_string()
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let (event_tx, event_rx) = mpsc::channel::<Event>();
    let tx_to_input_events: mpsc::Sender<Event> = event_tx.clone();
    thread::spawn(move || {
        handle_input_events(tx_to_input_events);
        });
    let username = prompt("give me your username: ");
    let mut list_state = ListState::default().with_selected(Some(0));
    ratatui::run(|terminal| App::new(username, list_state).run(terminal, event_rx));
//    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();
//    let ip = prompt("what IP are you trying to connect to?");
//    println!("connecting to {}", ip);
//    match TcpStream::connect(format!("{}:7878", ip)){
//        Ok(mut stream) => ratatui::run(|terminal| App::new(stream,listener,username).run(terminal, event_rx)),
//        Err(e) => panic!("{}",e)
//    }
}

struct App {
    exit: bool,
    input: String,
    character_index: usize,
    input_mode: InputMode,
    messages: Vec<String>,
    stream: TcpStream,
    listener: TcpListener,
    username: String,
    list_state: ListState,
}

enum InputMode {
    Normal,
    Editing,
    List,
    Connecting
}

// async events
enum Event {
    Input(crossterm::event::KeyEvent),
}

fn handle_input_events(tx: mpsc::Sender<Event>){
    loop {
        match crossterm::event::read().unwrap(){
            crossterm::event::Event::Key(key_event) => tx.send(Event::Input(key_event)).unwrap(),
            _ => {}
        }
    }
}

impl App {
    fn new(username: String, list_state: ListState) -> Self {
        Self {
            exit: false,
            input: String::new(),
            input_mode: InputMode::List,
            messages: Vec::new(),
            character_index: 0,
            username: username,
            list_state = list_state
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

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
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

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    const fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn submit_message(&mut self) {
        self.stream.write(&format!("{}: {}\r\n", self.username, self.input.clone()).into_bytes());
        self.messages.push(self.input.clone());
        self.input.clear();
        self.reset_cursor();
    }

    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> io::Result<()> {
        match self.input_mode {
            InputMode::List => match key_event.code{
                KeyCode::Down => self.list_state.select_next(),
                KeyCode::Up => self.list_state.select_previous(),
                KeyCode::Esc => { self.exit = true; }
                _ => {}
            }
            InputMode::Normal => match key_event.code {
                KeyCode::Char('e') => {
                    self.input_mode = InputMode::Editing;
                }
                KeyCode::Char('q') => {
                    self.exit = true;
                }
                _ => {}
            },
            InputMode::Editing if key_event.kind == KeyEventKind::Press => match key_event.code {
                KeyCode::Enter => self.submit_message(),
                KeyCode::Char(to_insert) => self.enter_char(to_insert),
                KeyCode::Backspace => self.delete_char(),
                KeyCode::Left => self.move_cursor_left(),
                KeyCode::Right => self.move_cursor_right(),
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                _ => {}
            },
            InputMode::Editing => {}
        }
        Ok(())
    }
    fn run(mut self, terminal: &mut DefaultTerminal, rx: mpsc::Receiver<Event>) -> Result<()> {
        while !self.exit {
            match rx.recv().unwrap(){
                    Event::Input(key_event) => self.handle_key_event(key_event)?,
                }
            terminal.draw(|frame| self.render(frame))?;
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
        ]);
        let [help_area, input_area, messages_area] = frame.area().layout(&layout);

        let (msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    "Press ".into(),
                    "q".bold(),
                    " to exit, ".into(),
                    "e".bold(),
                    " to start editing.".bold(),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Editing => (
                vec![
                    "Press ".into(),
                    "Esc".bold(),
                    " to stop editing, ".into(),
                    "Enter".bold(),
                    " to record the message".into(),
                ],
                Style::default(),
            ),
        };
        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);

        let input = Paragraph::new(self.input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, input_area);
        match self.input_mode {
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            InputMode::Normal => {}

            // Make the cursor visible and ask ratatui to put it at the specified coordinates after
            // rendering
            #[expect(clippy::cast_possible_truncation)]
            InputMode::Editing => frame.set_cursor_position(Position::new(
                // Draw the cursor at the current position in the input field.
                // This position can be controlled via the left and right arrow key
                input_area.x + self.character_index as u16 + 1,
                // Move one line down, from the border to the input line
                input_area.y + 1,
            )),
        }

        let messages: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let content = Line::from(Span::raw(format!("{i}: {m}")));
                ListItem::new(content)
            })
            .collect();
        let messages = List::new(messages).block(Block::bordered().title("Messages"));
        frame.render_widget(messages, messages_area);
    }
}

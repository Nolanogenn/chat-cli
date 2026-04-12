use std::{
    io::{BufReader, prelude::*},
    net::{TcpListener, TcpStream},
    time::Duration,
    thread
};

fn prompt(text: &str) -> String{
    print!("{}", text);
    std::io::stdout().flush().expect("Oups");
    let mut response = String::new();
    std::io::stdin()
        .read_line(&mut response)
        .expect("failed to get input");
    response.trim_end().to_string()
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:7878").unwrap();
    let ip = prompt("what IP are you trying to connect to?");
    println!("trying to connect to: {}", ip);
    match TcpStream::connect(format!("{}:7878", ip)){ 
        Ok(stream) => {
            println!("success");
                loop{
                    let message = prompt("> ");
                    println!("your message was {}", message);
                }
        }
        Err(e) => {println!("failed: {}", e); thread::sleep(Duration::from_secs(2))}
    }
}

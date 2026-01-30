mod state;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use rust_rush_trivia::protocol::*;
use rust_rush_trivia::{read_message, write_message};
use state::{ClientData, ClientState};
use std::io;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::error;

enum AppEvent {
    ServerMessage(ServerMessage),
    KeyPress(KeyCode),
    Tick,
}

async fn network_task(
    mut stream: TcpStream,
    tx: mpsc::Sender<AppEvent>,
) -> Result<()> {
    loop {
        match read_message::<_, ServerMessage>(&mut stream).await {
            Ok(msg) => {
                if tx.send(AppEvent::ServerMessage(msg)).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                error!("Network error: {}", e);
                break;
            }
        }
    }
    Ok(())
}

async fn send_message(stream: &mut TcpStream, msg: ClientMessage) -> Result<()> {
    write_message(stream, &msg).await
}

#[tokio::main]
async fn main() -> Result<()> {
    // Get server address and player name from command line
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        eprintln!("Usage: {} <server_ip:port> <player_name>", args[0]);
        eprintln!("Example: {} 192.168.1.100:8080 Alice", args[0]);
        std::process::exit(1);
    }
    
    let server_addr = &args[1];
    let player_name = args[2..].join(" ");
    
    println!("Connecting to {} as '{}'...", server_addr, player_name);
    
    // Connect to server
    let mut stream = TcpStream::connect(server_addr).await?;
    
    // Send join message
    send_message(&mut stream, ClientMessage::Join {
        player_name: player_name.clone(),
    }).await?;
    
    println!("Connected! Starting UI...");
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create client state
    let mut client = ClientData::new(player_name.clone());
    
    // Channel for app events
    let (tx, mut rx) = mpsc::channel::<AppEvent>(100);
    
    // Spawn network task
    let tx_clone = tx.clone();
    let stream_clone = stream.try_clone().await?;
    tokio::spawn(async move {
        if let Err(e) = network_task(stream_clone, tx_clone).await {
            error!("Network task error: {}", e);
        }
    });
    
    // Spawn input task
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            if event::poll(Duration::from_millis(100)).unwrap() {
                if let Event::Key(key) = event::read().unwrap() {
                    if tx_clone.send(AppEvent::KeyPress(key.code)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
    
    // Spawn tick task for timer updates
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if tx_clone.send(AppEvent::Tick).await.is_err() {
                break;
            }
        }
    });
    
    // Main event loop
    let mut should_quit = false;
    let mut question_number = 0u32;
    
    while !should_quit {
        // Render UI
        terminal.draw(|f| {
            ui::render_ui(f, &client);
        })?;
        
        // Handle events
        if let Some(event) = rx.recv().await {
            match event {
                AppEvent::ServerMessage(msg) => {
                    client.handle_server_message(msg);
                }
                
                AppEvent::KeyPress(key) => {
                    match key {
                        KeyCode::Esc => {
                            should_quit = true;
                            let _ = send_message(&mut stream, ClientMessage::Disconnect).await;
                        }
                        
                        KeyCode::Char('1') | KeyCode::Char('2') | KeyCode::Char('3') | KeyCode::Char('4') => {
                            if let ClientState::Question { number, answered: false, .. } = &client.state {
                                let choice = match key {
                                    KeyCode::Char('1') => 0,
                                    KeyCode::Char('2') => 1,
                                    KeyCode::Char('3') => 2,
                                    KeyCode::Char('4') => 3,
                                    _ => unreachable!(),
                                };
                                
                                question_number = *number;
                                
                                // Send answer
                                let timestamp = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs_f64();
                                
                                let _ = send_message(&mut stream, ClientMessage::Answer {
                                    question_number,
                                    choice_index: choice,
                                    timestamp,
                                }).await;
                                
                                // Update local state to show answered
                                if let ClientState::Question { ref mut answered, ref mut selected_index, .. } = client.state {
                                    *answered = true;
                                    *selected_index = Some(choice);
                                }
                            }
                        }
                        
                        _ => {}
                    }
                }
                
                AppEvent::Tick => {
                    // Just trigger a redraw for timer updates
                }
            }
        }
    }
    
    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    println!("Thanks for playing Rust Rush Trivia!");
    
    Ok(())
}

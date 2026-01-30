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
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tracing::error;

enum AppEvent {
    ServerMessage(ServerMessage),
    KeyPress(KeyCode),
    Tick,
}

async fn network_task(
    mut read_half: OwnedReadHalf,
    tx: mpsc::Sender<AppEvent>,
) -> Result<()> {
    loop {
        match read_message::<_, ServerMessage>(&mut read_half).await {
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

async fn send_message(write_half: &mut OwnedWriteHalf, msg: ClientMessage) -> Result<()> {
    write_message(write_half, &msg).await
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎮 Rust Rush Trivia Client");
    println!("==========================\n");

    // Prompt for server address
    print!("Enter server address (e.g., 192.168.1.100:8080): ");
    std::io::Write::flush(&mut std::io::stdout())?;

    let mut server_addr = String::new();
    std::io::stdin().read_line(&mut server_addr)?;
    let server_addr = server_addr.trim();

    // Prompt for player name
    print!("Enter your name: ");
    std::io::Write::flush(&mut std::io::stdout())?;

    let mut player_name = String::new();
    std::io::stdin().read_line(&mut player_name)?;
    let player_name = player_name.trim().to_string();

    if server_addr.is_empty() || player_name.is_empty() {
        eprintln!("Server address and player name are required!");
        std::process::exit(1);
    }

    println!("\nConnecting to {} as '{}'...", server_addr, player_name);
    
    // Connect to server
    let stream = TcpStream::connect(server_addr).await?;

    // Split stream into read and write halves
    let (read_half, mut write_half) = stream.into_split();

    // Send join message
    send_message(&mut write_half, ClientMessage::Join {
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
    tokio::spawn(async move {
        if let Err(e) = network_task(read_half, tx_clone).await {
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
                            let _ = send_message(&mut write_half, ClientMessage::Disconnect).await;
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
                                
                                let _ = send_message(&mut write_half, ClientMessage::Answer {
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

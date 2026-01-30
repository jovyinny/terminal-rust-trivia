mod game;
mod questions;
mod scoring;

use anyhow::Result;
use game::{Game, GameState};
use questions::QuestionBank;
use rust_rush_trivia::protocol::*;
use rust_rush_trivia::{read_message, write_message};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

type PlayerId = String;

enum GameEvent {
    PlayerJoined(PlayerId, String, mpsc::Sender<ServerMessage>),
    PlayerDisconnected(PlayerId),
    PlayerAnswer(PlayerId, u8),
    StartGame,
}

async fn handle_client(
    mut socket: TcpStream,
    addr: SocketAddr,
    game_tx: mpsc::Sender<GameEvent>,
) -> Result<()> {
    info!("New connection from {}", addr);
    
    // Read the initial Join message
    let join_msg: ClientMessage = read_message(&mut socket).await?;
    
    let player_name = match join_msg {
        ClientMessage::Join { player_name } => player_name,
        _ => {
            error!("Expected Join message, got something else");
            return Ok(());
        }
    };
    
    let player_id = format!("{}_{}", player_name, addr.port());
    info!("Player joined: {} (id: {})", player_name, player_id);
    
    // Send welcome message
    write_message(&mut socket, &ServerMessage::Welcome {
        player_id: player_id.clone(),
        server_version: "1.0.0".to_string(),
    }).await?;
    
    // Create channel for receiving broadcasts
    let (msg_tx, mut msg_rx) = mpsc::channel::<ServerMessage>(100);
    
    // Notify game loop about new player
    game_tx.send(GameEvent::PlayerJoined(player_id.clone(), player_name, msg_tx)).await?;
    
    let (mut read_half, mut write_half) = socket.into_split();
    
    // Spawn task to send messages to client
    let player_id_clone = player_id.clone();
    let game_tx_clone = game_tx.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            if let Err(e) = write_message(&mut write_half, &msg).await {
                warn!("Failed to send message to {}: {}", player_id_clone, e);
                break;
            }
        }
        let _ = game_tx_clone.send(GameEvent::PlayerDisconnected(player_id_clone)).await;
    });
    
    // Receive messages from client
    loop {
        match read_message::<_, ClientMessage>(&mut read_half).await {
            Ok(ClientMessage::Answer { question_number: _, choice_index, .. }) => {
                game_tx.send(GameEvent::PlayerAnswer(player_id.clone(), choice_index)).await?;
            }
            Ok(ClientMessage::Disconnect) => {
                info!("Player {} disconnected gracefully", player_id);
                break;
            }
            Ok(_) => {
                // Ignore other messages during game
            }
            Err(e) => {
                warn!("Error reading from {}: {}", player_id, e);
                break;
            }
        }
    }
    
    game_tx.send(GameEvent::PlayerDisconnected(player_id.clone())).await?;
    send_task.abort();
    
    Ok(())
}

async fn broadcast_to_players(
    players: &HashMap<PlayerId, mpsc::Sender<ServerMessage>>,
    message: ServerMessage,
) {
    for (player_id, tx) in players.iter() {
        if let Err(e) = tx.send(message.clone()).await {
            warn!("Failed to send to player {}: {}", player_id, e);
        }
    }
}

async fn game_loop(mut game_rx: mpsc::Receiver<GameEvent>) -> Result<()> {
    let questions = match QuestionBank::load_or_fetch("questions.json", 10).await {
        Ok(bank) => bank.select_questions(10),
        Err(e) => {
            error!("❌ Failed to load questions: {}", e);
            error!("Unable to start game. Shutting down server...");
            std::process::exit(1);
        }
    };

    info!("Loaded {} questions for the game", questions.len());
    
    let mut game = Game::new(questions);
    let mut player_channels: HashMap<PlayerId, mpsc::Sender<ServerMessage>> = HashMap::new();
    
    info!("Game server ready. Waiting for players to join...");
    info!("Press Ctrl+C to start the game once enough players have joined.");
    
    let mut game_started = false;

    loop {
        tokio::select! {
            Some(event) = game_rx.recv() => {
                match event {
                    GameEvent::PlayerJoined(id, name, tx) => {
                        if game.state == GameState::Lobby {
                            game.add_player(id.clone(), name);
                            player_channels.insert(id.clone(), tx);
                            
                            // Broadcast lobby update
                            let players = game.get_lobby_players();
                            broadcast_to_players(&player_channels, ServerMessage::LobbyUpdate {
                                players,
                                total_count: player_channels.len() as u32,
                            }).await;
                            
                            info!("Total players in lobby: {}", player_channels.len());
                        } else {
                            // Game already started, send error
                            let _ = tx.send(ServerMessage::Error {
                                message: "Game already in progress".to_string(),
                            }).await;
                        }
                    }
                    
                    GameEvent::PlayerDisconnected(id) => {
                        game.remove_player(&id);
                        player_channels.remove(&id);
                        info!("Player {} disconnected", id);
                        
                        if game.state == GameState::Lobby {
                            let players = game.get_lobby_players();
                            broadcast_to_players(&player_channels, ServerMessage::LobbyUpdate {
                                players,
                                total_count: player_channels.len() as u32,
                            }).await;
                        }
                    }
                    
                    GameEvent::PlayerAnswer(id, answer_idx) => {
                        if let GameState::Question(_) = game.state {
                            game.submit_answer(&id, answer_idx);
                        }
                    }
                    
                    GameEvent::StartGame => {
                        if game.players.len() >= 2 {
                            info!("Starting game with {} players!", game.players.len());
                            game_started = true;
                            game.start_game();
                            
                            broadcast_to_players(&player_channels, ServerMessage::GameStart {
                                total_questions: game.total_questions,
                            }).await;
                            
                            sleep(Duration::from_secs(2)).await;
                            
                            // Start first question
                            if let Some(question) = game.get_current_question() {
                                let question_text = question.text.clone();
                                let question_options = question.options.clone();
                                game.start_question();
                                broadcast_to_players(&player_channels, ServerMessage::Question {
                                    number: 1,
                                    total: game.total_questions,
                                    text: question_text,
                                    options: question_options,
                                    time_limit: 15,
                                }).await;
                            }
                        } else {
                            info!("Not enough players to start (need at least 2)");
                        }
                    }
                }
            }
            
            _ = sleep(Duration::from_millis(100)), if game_started => {
                match game.state {
                    GameState::Question(qnum) => {
                        // Check if time expired or all answered
                        if let Some(start_time) = game.question_start_time {
                            let elapsed = start_time.elapsed();
                            
                            if elapsed > Duration::from_secs(15) || game.all_answered() {
                                // Time to reveal results
                                let results = game.calculate_results();
                                let leaderboard = game.get_leaderboard();
                                let correct_idx = game.get_current_question().unwrap().correct_index;
                                
                                broadcast_to_players(&player_channels, ServerMessage::QuestionResult {
                                    correct_index: correct_idx,
                                    player_results: results,
                                    leaderboard,
                                }).await;
                                
                                game.state = GameState::Revealing(qnum);
                            }
                        }
                    }
                    
                    GameState::Revealing(qnum) => {
                        // Wait 4 seconds before next question
                        sleep(Duration::from_secs(4)).await;
                        
                        if game.advance_to_next_question() {
                            if game.state == GameState::Break {
                                info!("Brief break...");
                                sleep(Duration::from_secs(5)).await;
                                game.state = GameState::Question((qnum / 4 + 1) * 4 + 1);
                            }
                            
                            if let GameState::Question(next_num) = game.state {
                                if let Some(question) = game.get_current_question() {
                                    let question_text = question.text.clone();
                                    let question_options = question.options.clone();
                                    game.start_question();
                                    broadcast_to_players(&player_channels, ServerMessage::Question {
                                        number: next_num,
                                        total: game.total_questions,
                                        text: question_text,
                                        options: question_options,
                                        time_limit: 15,
                                    }).await;
                                }
                            }
                        } else {
                            // Game ended
                            let final_leaderboard = game.get_leaderboard();
                            let stats = game.get_game_stats();
                            
                            broadcast_to_players(&player_channels, ServerMessage::GameEnd {
                                final_leaderboard,
                                stats,
                            }).await;
                            
                            info!("Game ended! Thanks for playing!");
                            sleep(Duration::from_secs(10)).await;
                            break;
                        }
                    }
                    
                    _ => {}
                }
            }
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("🎮 Rust Rush Trivia Server Starting...");
    
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    let local_addr = listener.local_addr()?;
    
    info!("✅ Server listening on {}", local_addr);
    info!("📢 Players can connect using: <server-ip>:8080");
    info!("");
    info!("Waiting for players to join...");
    info!("The game will auto-start after 30 seconds if 2+ players are ready,");
    info!("or you can manually trigger start by sending SIGTERM (this is for demo purposes).");
    
    let (game_tx, game_rx) = mpsc::channel::<GameEvent>(100);
    
    // Spawn game loop
    tokio::spawn(async move {
        if let Err(e) = game_loop(game_rx).await {
            error!("Game loop error: {}", e);
        }
    });
    
    // Spawn a task to listen for manual start command
    let game_tx_for_start = game_tx.clone();
    tokio::spawn(async move {
        // Wait for Ctrl+C or specific signal
        tokio::signal::ctrl_c().await.ok();
        info!("Received start signal, beginning game...");
        let _ = game_tx_for_start.send(GameEvent::StartGame).await;
    });
    
    // Accept client connections
    loop {
        let (socket, addr) = listener.accept().await?;
        let game_tx = game_tx.clone();
        
        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, addr, game_tx).await {
                error!("Client handler error: {}", e);
            }
        });
    }
}

use rust_rush_trivia::protocol::*;

#[derive(Debug, Clone)]
pub enum ClientState {
    Connecting,
    Lobby(Vec<PlayerInfo>),
    WaitingForGame,
    Question {
        number: u32,
        total: u32,
        text: String,
        options: Vec<String>,
        time_limit: u32,
        answered: bool,
        selected_index: Option<u8>,
    },
    QuestionResult {
        correct_index: u8,
        player_results: Vec<PlayerResult>,
        leaderboard: Vec<LeaderboardEntry>,
    },
    GameEnd {
        final_leaderboard: Vec<LeaderboardEntry>,
        stats: GameStats,
    },
    Error(String),
}

impl Default for ClientState {
    fn default() -> Self {
        ClientState::Connecting
    }
}

pub struct ClientData {
    pub state: ClientState,
    pub player_id: Option<String>,
    pub player_name: String,
    pub question_start_time: Option<std::time::Instant>,
}

impl ClientData {
    pub fn new(player_name: String) -> Self {
        Self {
            state: ClientState::Connecting,
            player_id: None,
            player_name,
            question_start_time: None,
        }
    }
    
    pub fn handle_server_message(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::Welcome { player_id, .. } => {
                self.player_id = Some(player_id);
            }
            
            ServerMessage::LobbyUpdate { players, .. } => {
                self.state = ClientState::Lobby(players);
            }
            
            ServerMessage::GameStart { .. } => {
                self.state = ClientState::WaitingForGame;
            }
            
            ServerMessage::Question { number, total, text, options, time_limit } => {
                self.state = ClientState::Question {
                    number,
                    total,
                    text,
                    options,
                    time_limit,
                    answered: false,
                    selected_index: None,
                };
                self.question_start_time = Some(std::time::Instant::now());
            }
            
            ServerMessage::QuestionResult { correct_index, player_results, leaderboard } => {
                self.state = ClientState::QuestionResult {
                    correct_index,
                    player_results,
                    leaderboard,
                };
            }
            
            ServerMessage::GameEnd { final_leaderboard, stats } => {
                self.state = ClientState::GameEnd {
                    final_leaderboard,
                    stats,
                };
            }
            
            ServerMessage::Error { message } => {
                self.state = ClientState::Error(message);
            }
        }
    }
    
    pub fn get_time_remaining(&self) -> f64 {
        if let Some(start) = self.question_start_time {
            let elapsed = start.elapsed().as_secs_f64();
            (15.0 - elapsed).max(0.0)
        } else {
            15.0
        }
    }
}

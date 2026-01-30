use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: String,
    pub name: String,
    pub score: u32,
    pub streak: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerResult {
    pub player_id: String,
    pub player_name: String,
    pub answered: bool,
    pub correct: bool,
    pub answer_time: Option<f64>,
    pub points_earned: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub player_id: String,
    pub player_name: String,
    pub score: u32,
    pub streak: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStats {
    pub fastest_answer: Option<(String, f64)>,
    pub longest_streak: Option<(String, u32)>,
    pub perfect_score: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    Welcome {
        player_id: String,
        server_version: String,
    },
    LobbyUpdate {
        players: Vec<PlayerInfo>,
        total_count: u32,
    },
    GameStart {
        total_questions: u32,
    },
    Question {
        number: u32,
        total: u32,
        text: String,
        options: Vec<String>,
        time_limit: u32,
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
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Join { player_name: String },
    Answer { question_number: u32, choice_index: u8, timestamp: f64 },
    Ready,
    Disconnect,
}

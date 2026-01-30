use rust_rush_trivia::protocol::*;
use crate::questions::Question;
use crate::scoring;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub score: u32,
    pub streak: u32,
    pub connected: bool,
    pub current_answer: Option<(u8, f64)>, // (answer_index, time_remaining)
}

#[derive(Debug, PartialEq)]
pub enum GameState {
    Lobby,
    Question(u32),      // Current question number
    Revealing(u32),     // Revealing results for question number
    Break,              // Brief break between question sets
    Ended,
}

pub struct Game {
    pub state: GameState,
    pub players: HashMap<String, Player>,
    pub questions: Vec<Question>,
    pub current_question_idx: usize,
    pub question_start_time: Option<Instant>,
    pub total_questions: u32,
}

impl Game {
    pub fn new(questions: Vec<Question>) -> Self {
        let total = questions.len() as u32;
        Self {
            state: GameState::Lobby,
            players: HashMap::new(),
            questions,
            current_question_idx: 0,
            question_start_time: None,
            total_questions: total,
        }
    }
    
    pub fn add_player(&mut self, id: String, name: String) {
        self.players.insert(id.clone(), Player {
            id,
            name,
            score: 0,
            streak: 0,
            connected: true,
            current_answer: None,
        });
    }
    
    pub fn remove_player(&mut self, id: &str) {
        if let Some(player) = self.players.get_mut(id) {
            player.connected = false;
        }
    }
    
    pub fn start_game(&mut self) {
        if self.players.len() >= 2 {
            self.state = GameState::Question(1);
            self.current_question_idx = 0;
        }
    }
    
    pub fn start_question(&mut self) {
        self.question_start_time = Some(Instant::now());
        // Clear all previous answers
        for player in self.players.values_mut() {
            player.current_answer = None;
        }
    }
    
    pub fn submit_answer(&mut self, player_id: &str, answer_idx: u8) -> bool {
        if let Some(player) = self.players.get_mut(player_id) {
            if player.current_answer.is_none() {
                if let Some(start_time) = self.question_start_time {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let time_remaining = (15.0 - elapsed).max(0.0);
                    player.current_answer = Some((answer_idx, time_remaining));
                    return true;
                }
            }
        }
        false
    }
    
    pub fn all_answered(&self) -> bool {
        let connected_count = self.players.values().filter(|p| p.connected).count();
        let answered_count = self.players.values()
            .filter(|p| p.connected && p.current_answer.is_some())
            .count();
        connected_count > 0 && answered_count == connected_count
    }
    
    pub fn calculate_results(&mut self) -> Vec<PlayerResult> {
        let current_question = &self.questions[self.current_question_idx];
        let correct_idx = current_question.correct_index;
        
        let mut results = Vec::new();
        
        for player in self.players.values_mut() {
            if !player.connected {
                continue;
            }
            
            let (answered, correct, answer_time, points_earned) = if let Some((answer_idx, time_remaining)) = player.current_answer {
                let is_correct = answer_idx == correct_idx;
                
                let points = if is_correct {
                    let earned = scoring::calculate_score(time_remaining, player.streak);
                    player.score += earned;
                    player.streak += 1;
                    earned
                } else {
                    player.streak = 0;
                    0
                };
                
                (true, is_correct, Some(15.0 - time_remaining), points)
            } else {
                player.streak = 0;
                (false, false, None, 0)
            };
            
            results.push(PlayerResult {
                player_id: player.id.clone(),
                player_name: player.name.clone(),
                answered,
                correct,
                answer_time,
                points_earned,
            });
        }
        
        results
    }
    
    pub fn get_leaderboard(&self) -> Vec<LeaderboardEntry> {
        let mut entries: Vec<_> = self.players.values()
            .filter(|p| p.connected)
            .map(|p| LeaderboardEntry {
                rank: 0,
                player_id: p.id.clone(),
                player_name: p.name.clone(),
                score: p.score,
                streak: p.streak,
            })
            .collect();
        
        entries.sort_by(|a, b| b.score.cmp(&a.score).then(a.player_name.cmp(&b.player_name)));
        
        for (idx, entry) in entries.iter_mut().enumerate() {
            entry.rank = (idx + 1) as u32;
        }
        
        entries
    }
    
    pub fn advance_to_next_question(&mut self) -> bool {
        self.current_question_idx += 1;
        
        if self.current_question_idx >= self.questions.len() {
            self.state = GameState::Ended;
            false
        } else {
            let next_number = (self.current_question_idx + 1) as u32;
            
            // Check if we should have a break (every 4 questions)
            if next_number % 4 == 1 && next_number > 1 {
                self.state = GameState::Break;
            } else {
                self.state = GameState::Question(next_number);
            }
            true
        }
    }
    
    pub fn get_current_question(&self) -> Option<&Question> {
        self.questions.get(self.current_question_idx)
    }
    
    pub fn get_game_stats(&self) -> GameStats {
        let fastest_answer: Option<(String, f64)> = None;
        let mut longest_streak: Option<(String, u32)> = None;
        let mut perfect_score: Vec<String> = Vec::new();
        
        for player in self.players.values() {
            if !player.connected {
                continue;
            }
            
            // Check for perfect score (all questions correct)
            if player.streak == self.total_questions {
                perfect_score.push(player.name.clone());
            }
            
            // Track longest streak
            if let Some((_, max_streak)) = longest_streak {
                if player.streak > max_streak {
                    longest_streak = Some((player.name.clone(), player.streak));
                }
            } else if player.streak > 0 {
                longest_streak = Some((player.name.clone(), player.streak));
            }
        }
        
        GameStats {
            fastest_answer,
            longest_streak,
            perfect_score,
        }
    }
    
    pub fn get_lobby_players(&self) -> Vec<PlayerInfo> {
        self.players.values()
            .filter(|p| p.connected)
            .map(|p| PlayerInfo {
                id: p.id.clone(),
                name: p.name.clone(),
                score: p.score,
                streak: p.streak,
            })
            .collect()
    }
}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: u32,
    pub text: String,
    pub options: Vec<String>,
    pub correct_index: u8,
    #[serde(default)]
    pub category: String,
}

#[derive(Debug, Deserialize)]
pub struct QuestionBank {
    pub questions: Vec<Question>,
}

impl QuestionBank {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let bank: QuestionBank = serde_json::from_str(&content)?;
        Ok(bank)
    }
    
    pub fn select_questions(&self, count: usize) -> Vec<Question> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;
        
        let mut rng = thread_rng();
        let mut questions = self.questions.clone();
        questions.shuffle(&mut rng);
        questions.into_iter().take(count).collect()
    }
}

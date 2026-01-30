use anyhow::{Context, Result};
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

// Open Trivia Database API response structures
#[derive(Debug, Deserialize)]
struct OpenTriviaResponse {
    response_code: i32,
    results: Vec<OpenTriviaQuestion>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenTriviaQuestion {
    category: String,
    #[serde(rename = "type")]
    question_type: String,
    difficulty: String,
    question: String,
    correct_answer: String,
    incorrect_answers: Vec<String>,
}

impl QuestionBank {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let bank: QuestionBank = serde_json::from_str(&content)?;
        Ok(bank)
    }
    
    pub async fn load_or_fetch(path: &str, count: usize) -> Result<Self> {
        // Try to load from local file first
        match Self::load_from_file(path) {
            Ok(bank) => {
                tracing::info!("✅ Loaded questions from local file: {}", path);
                Ok(bank)
            }
            Err(e) => {
                tracing::warn!("⚠️  Local questions file not found: {}", e);
                tracing::info!("🌐 Attempting to fetch questions from Open Trivia Database...");

                Self::fetch_from_api(count).await
            }
        }
    }

    async fn fetch_from_api(count: usize) -> Result<Self> {
        let url = format!(
            "https://opentdb.com/api.php?amount={}&type=multiple",
            count
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let response = client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to Open Trivia Database. Please check your internet connection.")?;

        let trivia_response: OpenTriviaResponse = response
            .json()
            .await
            .context("Failed to parse response from Open Trivia Database")?;

        if trivia_response.response_code != 0 {
            anyhow::bail!("Open Trivia Database returned error code: {}", trivia_response.response_code);
        }

        if trivia_response.results.is_empty() {
            anyhow::bail!("No questions received from Open Trivia Database");
        }

        // Convert Open Trivia questions to our format
        let questions: Vec<Question> = trivia_response
            .results
            .into_iter()
            .enumerate()
            .map(|(idx, q)| {
                use rand::seq::SliceRandom;
                use rand::thread_rng;

                // Decode HTML entities
                let text = html_escape::decode_html_entities(&q.question).to_string();
                let correct = html_escape::decode_html_entities(&q.correct_answer).to_string();

                let mut options: Vec<String> = q
                    .incorrect_answers
                    .iter()
                    .map(|s| html_escape::decode_html_entities(s).to_string())
                    .collect();

                // Insert correct answer at a random position
                let mut rng = thread_rng();
                let correct_index = rand::Rng::gen_range(&mut rng, 0..=options.len());
                options.insert(correct_index, correct.clone());

                // Shuffle to be extra safe
                options.shuffle(&mut rng);
                let correct_index = options.iter().position(|s| s == &correct).unwrap();

                Question {
                    id: idx as u32 + 1,
                    text,
                    options,
                    correct_index: correct_index as u8,
                    category: q.category,
                }
            })
            .collect();

        tracing::info!("✅ Successfully fetched {} questions from Open Trivia Database", questions.len());

        Ok(QuestionBank { questions })
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

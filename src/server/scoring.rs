/// Calculate score for a correct answer
/// 
/// Base: 100 points
/// Time bonus: 0-50 points based on time remaining (15 second timer)
/// Streak bonus: +10% for 2 correct, +20% for 3+ correct
pub fn calculate_score(time_remaining: f64, streak: u32) -> u32 {
    let base = 100u32;
    let time_bonus = ((time_remaining / 15.0) * 50.0).max(0.0).min(50.0) as u32;
    
    let streak_multiplier = match streak {
        0..=1 => 1.0,
        2 => 1.1,      // +10%
        _ => 1.2,      // +20% capped
    };
    
    ((base + time_bonus) as f64 * streak_multiplier) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_score_instant_answer() {
        // Instant answer, no streak
        assert_eq!(calculate_score(15.0, 0), 150);
        
        // Instant answer, 2-streak
        assert_eq!(calculate_score(15.0, 2), 165);
        
        // Instant answer, 3+ streak
        assert_eq!(calculate_score(15.0, 3), 180);
    }
    
    #[test]
    fn test_score_half_time() {
        // 7.5 seconds remaining, no streak
        assert_eq!(calculate_score(7.5, 0), 125);
    }
    
    #[test]
    fn test_score_last_second() {
        // 1 second remaining, no streak
        let score = calculate_score(1.0, 0);
        assert!(score >= 103 && score <= 107);
    }
}

use crate::state::{ClientData, ClientState};
use rust_rush_trivia::protocol::*;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render_ui(f: &mut Frame, client: &ClientData) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),      // Content
            Constraint::Length(3),  // Footer
        ])
        .split(f.area());
    
    // Header
    render_header(f, chunks[0], client);
    
    // Main content based on state
    match &client.state {
        ClientState::Connecting => render_connecting(f, chunks[1]),
        ClientState::Lobby(players) => render_lobby(f, chunks[1], players),
        ClientState::WaitingForGame => render_waiting_for_game(f, chunks[1]),
        ClientState::Question { number, total, text, options, answered, selected_index, .. } => {
            render_question(f, chunks[1], *number, *total, text, options, *answered, *selected_index, client.get_time_remaining());
        }
        ClientState::QuestionResult { correct_index, player_results, leaderboard } => {
            render_question_result(f, chunks[1], *correct_index, player_results, leaderboard, &client.player_id);
        }
        ClientState::GameEnd { final_leaderboard, stats } => {
            render_game_end(f, chunks[1], final_leaderboard, stats, &client.player_id);
        }
        ClientState::Error(msg) => render_error(f, chunks[1], msg),
    }
    
    // Footer
    render_footer(f, chunks[2], &client.state);
}

fn render_header(f: &mut Frame, area: Rect, client: &ClientData) {
    let title = if let Some(_id) = &client.player_id {
        format!("🎮 Rust Rush Trivia - Player: {} ", client.player_name)
    } else {
        "🎮 Rust Rush Trivia - Connecting...".to_string()
    };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan))
        .title(title);
    
    let paragraph = Paragraph::new("").block(block);
    f.render_widget(paragraph, area);
}

fn render_footer(f: &mut Frame, area: Rect, state: &ClientState) {
    let help_text = match state {
        ClientState::Question { answered: false, .. } => "Press 1-4 to answer | ESC to quit",
        ClientState::Lobby(_) => "Waiting for game to start... | ESC to quit",
        _ => "ESC to quit",
    };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::DarkGray));
    
    let paragraph = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Center);
    
    f.render_widget(paragraph, area);
}

fn render_connecting(f: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from("Connecting to server..."),
        Line::from(""),
    ];
    
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Status"));
    
    f.render_widget(paragraph, area);
}

fn render_lobby(f: &mut Frame, area: Rect, players: &[PlayerInfo]) {
    let mut items: Vec<ListItem> = players.iter()
        .map(|p| {
            let content = format!("👤 {}", p.name);
            ListItem::new(content).style(Style::default().fg(Color::Green))
        })
        .collect();
    
    if items.is_empty() {
        items.push(ListItem::new("No players yet...").style(Style::default().fg(Color::DarkGray)));
    }
    
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!("Lobby - {} Players Connected", players.len())));
    
    f.render_widget(list, area);
}

fn render_waiting_for_game(f: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("Game Starting Soon!", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("Get ready to answer questions!"),
    ];
    
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    
    f.render_widget(paragraph, area);
}

fn render_question(
    f: &mut Frame,
    area: Rect,
    number: u32,
    total: u32,
    text: &str,
    options: &[String],
    answered: bool,
    selected_index: Option<u8>,
    time_remaining: f64,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Timer and question number
            Constraint::Length(6),  // Question text
            Constraint::Min(0),     // Options
        ])
        .split(area);
    
    // Timer and question number
    let timer_text = if answered {
        format!("✓ Answered!")
    } else {
        format!("⏱  Time: {:.1}s", time_remaining)
    };
    
    let timer_color = if time_remaining > 10.0 {
        Color::Green
    } else if time_remaining > 5.0 {
        Color::Yellow
    } else {
        Color::Red
    };
    
    let header = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Question {} of {}", number, total),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(timer_text, Style::default().fg(timer_color).add_modifier(Modifier::BOLD))),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    
    f.render_widget(header, chunks[0]);
    
    // Question text
    let question_block = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("Question"));
    
    f.render_widget(question_block, chunks[1]);
    
    // Options
    let option_items: Vec<ListItem> = options.iter()
        .enumerate()
        .map(|(i, opt)| {
            let prefix = format!("{}. ", i + 1);
            let style = if answered && Some(i as u8) == selected_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            
            ListItem::new(format!("{}{}", prefix, opt)).style(style)
        })
        .collect();
    
    let options_list = List::new(option_items)
        .block(Block::default().borders(Borders::ALL).title("Options (Press 1-4)"));
    
    f.render_widget(options_list, chunks[2]);
}

fn render_question_result(
    f: &mut Frame,
    area: Rect,
    correct_index: u8,
    player_results: &[PlayerResult],
    leaderboard: &[LeaderboardEntry],
    my_player_id: &Option<String>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Correct answer
            Constraint::Percentage(50),  // Player results
            Constraint::Percentage(50),  // Leaderboard
        ])
        .split(area);
    
    // Correct answer
    let correct_text = format!("✓ Correct Answer: Option {}", correct_index + 1);
    let correct_para = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(correct_text, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    
    f.render_widget(correct_para, chunks[0]);
    
    // Player results
    let result_items: Vec<ListItem> = player_results.iter()
        .map(|r| {
            let status = if r.correct { "✓" } else if r.answered { "✗" } else { "⊗" };
            let points = if r.points_earned > 0 {
                format!(" (+{})", r.points_earned)
            } else {
                String::new()
            };
            
            let color = if r.correct { Color::Green } else { Color::Red };
            let is_me = my_player_id.as_ref().map_or(false, |id| id == &r.player_id);
            let prefix = if is_me { "→ " } else { "  " };
            
            ListItem::new(format!("{}{} {}{}", prefix, status, r.player_name, points))
                .style(Style::default().fg(color))
        })
        .collect();
    
    let results_list = List::new(result_items)
        .block(Block::default().borders(Borders::ALL).title("Results"));
    
    f.render_widget(results_list, chunks[1]);
    
    // Leaderboard
    render_leaderboard_widget(f, chunks[2], leaderboard, my_player_id);
}

fn render_leaderboard_widget(
    f: &mut Frame,
    area: Rect,
    leaderboard: &[LeaderboardEntry],
    my_player_id: &Option<String>,
) {
    let items: Vec<ListItem> = leaderboard.iter()
        .map(|entry| {
            let medal = match entry.rank {
                1 => "🥇",
                2 => "🥈",
                3 => "🥉",
                _ => "  ",
            };
            
            let is_me = my_player_id.as_ref().map_or(false, |id| id == &entry.player_id);
            let style = if is_me {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if entry.rank <= 3 {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            
            let prefix = if is_me { "→ " } else { "  " };
            let streak = if entry.streak > 0 {
                format!(" 🔥{}", entry.streak)
            } else {
                String::new()
            };
            
            ListItem::new(format!(
                "{}{} {}. {} - {} pts{}",
                prefix, medal, entry.rank, entry.player_name, entry.score, streak
            ))
            .style(style)
        })
        .collect();
    
    let leaderboard_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Leaderboard"));
    
    f.render_widget(leaderboard_list, area);
}

fn render_game_end(
    f: &mut Frame,
    area: Rect,
    leaderboard: &[LeaderboardEntry],
    stats: &GameStats,
    my_player_id: &Option<String>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Title
            Constraint::Percentage(60),  // Final leaderboard
            Constraint::Percentage(40),  // Stats
        ])
        .split(area);
    
    // Title
    let title_para = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled("🏆 GAME OVER 🏆", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    
    f.render_widget(title_para, chunks[0]);
    
    // Final leaderboard
    render_leaderboard_widget(f, chunks[1], leaderboard, my_player_id);
    
    // Stats
    let mut stat_lines = vec![Line::from("")];
    
    if let Some((name, streak)) = &stats.longest_streak {
        stat_lines.push(Line::from(format!("🔥 Longest Streak: {} ({})", name, streak)));
    }
    
    if !stats.perfect_score.is_empty() {
        stat_lines.push(Line::from(format!("⭐ Perfect Score: {}", stats.perfect_score.join(", "))));
    }
    
    let stats_para = Paragraph::new(stat_lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Game Stats"));
    
    f.render_widget(stats_para, chunks[2]);
}

fn render_error(f: &mut Frame, area: Rect, message: &str) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("❌ Error", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(message),
    ];
    
    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    
    f.render_widget(paragraph, area);
}

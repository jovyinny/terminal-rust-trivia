# 🎮 Rust Rush Trivia

A fast-paced, multiplayer trivia game built in Rust for Innovation Day demos. Play with 10-25 teammates on the same local network in an exciting 8-12 minute game session!

## 🌟 Features

- **Real-time multiplayer**: 10-25 concurrent players
- **Fast-paced gameplay**: 15-second timer per question
- **Smart scoring**: Base points + time bonus + streak multipliers
- **Terminal UI**: Beautiful TUI using ratatui
- **LAN-based**: No internet required (works offline)
- **Production-ready**: Proper error handling, connection management

## 🎯 Game Rules

### Scoring System
- **Base score**: 100 points for correct answers
- **Time bonus**: +0 to +50 points based on answer speed
- **Streak bonus**: 
  - 2 correct in a row: +10% 
  - 3+ correct: +20% (capped)
- Wrong answers: 0 points (no penalties)

### Game Flow
1. **Lobby**: Players join and wait for game to start
2. **Questions**: 8-12 questions, 15 seconds each
3. **Results**: See correct answer and updated leaderboard after each question
4. **Breaks**: 5-second break every 4 questions
5. **Game End**: Final leaderboard and stats

## 📋 Requirements

- Rust 1.70+ (install from https://rustup.rs/)
- Local network (Wi-Fi or Ethernet)
- Terminal that supports Unicode and colors

## 🚀 Quick Start

### 1. Clone and Build

```bash
cd rust-rush-trivia
cargo build --release
```

### 2. Start the Server

On one machine (the "host" machine), run:

```bash
cargo run --release --bin server
```

The server will display:
```
✅ Server listening on 0.0.0.0:8080
📢 Players can connect using: <server-ip>:8080
```

**Find your server IP address:**

- **Linux/Mac**: `ip addr show` or `ifconfig`
- **Windows**: `ipconfig`

Look for an IP like `192.168.1.100` (not 127.0.0.1)

### 3. Players Connect

Each player runs on their own machine:

```bash
cargo run --release --bin client <server-ip>:8080 "PlayerName"
```

**Examples:**
```bash
cargo run --release --bin client 192.168.1.100:8080 "Alice"
cargo run --release --bin client 192.168.1.100:8080 "Bob"
cargo run --release --bin client 10.0.0.50:8080 "Charlie"
```

### 4. Start the Game

The game will **auto-start after 30 seconds** once 2+ players are connected.

Or manually trigger start by pressing **Ctrl+C** on the server terminal.

### 5. Play!

- Press **1-4** to answer questions
- Watch the timer count down
- See live leaderboard updates
- Press **ESC** to quit anytime

## 🎮 Gameplay Tips

- **Answer fast** for time bonuses (up to +50 points)
- **Build streaks** for multipliers (+10% or +20%)
- **Watch the timer** - it changes color as time runs out
- **Track your rank** - leaderboard shows after every question

## 🛠️ Project Structure

```
rust-rush-trivia/
├── Cargo.toml              # Dependencies
├── questions.json          # Question bank (customize this!)
├── README.md              # This file
└── src/
    ├── lib.rs             # Shared utilities (message framing)
    ├── protocol.rs        # Message definitions
    ├── server/
    │   ├── main.rs        # Server entry point
    │   ├── game.rs        # Game state machine
    │   ├── questions.rs   # Question loading
    │   └── scoring.rs     # Score calculation
    └── client/
        ├── main.rs        # Client entry point
        ├── state.rs       # Client state management
        └── ui.rs          # Terminal UI (ratatui)
```

## 📝 Customizing Questions

Edit `questions.json` to add your own questions:

```json
{
  "questions": [
    {
      "id": 1,
      "text": "Your question here?",
      "options": ["Option A", "Option B", "Option C", "Option D"],
      "correct_index": 0,
      "category": "custom"
    }
  ]
}
```

**Tips:**
- Keep questions concise (fits on one line)
- Use exactly 4 options
- `correct_index` is 0-based (0 = first option)
- Add 10-25 questions for a full game

## 🐛 Troubleshooting

### "Connection refused"
- Check server is running
- Verify IP address is correct
- Ensure firewall allows port 8080
- Make sure both machines are on the same network

### "Game already in progress"
- Wait for current game to end
- Server must be restarted for new game

### Terminal display issues
- Use a modern terminal (iTerm2, Windows Terminal, etc.)
- Ensure terminal supports Unicode
- Try resizing terminal window

### Players can't connect
```bash
# Linux: Allow port 8080
sudo ufw allow 8080/tcp

# macOS: Check System Preferences → Firewall
# Windows: Check Windows Defender Firewall
```

## 🎯 Demo Tips

### Before the Demo
1. Test with 2-3 players first
2. Customize questions for your audience
3. Ensure Wi-Fi is stable
4. Have server IP written down clearly

### During the Demo
1. Start server 5 minutes early
2. Write server IP on whiteboard
3. Help first few players connect
4. Let game auto-start at 30 seconds
5. Show leaderboard on projector (if available)

### For Best Impact
- Use company-specific trivia questions
- Mix difficulty levels
- Keep energy high with commentary
- Celebrate the winner!

## 🔧 Advanced Configuration

### Change Server Port

Edit `src/server/main.rs`:
```rust
let listener = TcpListener::bind("0.0.0.0:9000").await?;
```

### Adjust Timing

Edit `src/server/main.rs`:
```rust
// Question timer (currently 15 seconds)
if elapsed > Duration::from_secs(15) || game.all_answered() {

// Result display time (currently 4 seconds)
sleep(Duration::from_secs(4)).await;

// Break duration (currently 5 seconds)
sleep(Duration::from_secs(5)).await;
```

### Change Number of Questions

Edit `src/server/main.rs`:
```rust
.select_questions(10)  // Change to desired number
```

## 📊 Technical Details

- **Protocol**: TCP with length-prefixed JSON messages
- **Concurrency**: Tokio async runtime
- **UI Framework**: ratatui (terminal UI)
- **Serialization**: serde + serde_json
- **Architecture**: Actor-like message passing

## 🤝 Contributing

This is a demo project for Innovation Day. Feel free to:
- Add more questions
- Improve the UI
- Add new game modes
- Enhance scoring mechanics

## 📜 License

MIT License - feel free to use and modify for your Innovation Day demos!

## 🎉 Credits

Built with Rust 🦀 for Innovation Day demos.

Crates used:
- `tokio` - Async runtime
- `ratatui` - Terminal UI
- `serde` - Serialization
- `crossterm` - Terminal control

---

**Ready to play? Let's Rust Rush!** 🚀

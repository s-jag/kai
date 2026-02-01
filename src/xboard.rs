/// XBoard/WinBoard protocol implementation
///
/// This module implements the XBoard (also known as WinBoard or CECP - Chess Engine
/// Communication Protocol) for compatibility with XBoard-based chess GUIs.
///
/// Reference: https://www.gnu.org/software/xboard/engine-intf.html

use crate::magic::init_magics;
use crate::moves::Move;
use crate::position::Position;
use crate::tt::TranspositionTable;
use crate::types::Color;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Global stop flag for search
static STOP_FLAG: AtomicBool = AtomicBool::new(false);

/// XBoard protocol version we support
const PROTOCOL_VERSION: u32 = 2;

/// XBoard engine state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineMode {
    /// Waiting for commands, not thinking
    Force,
    /// Playing as specified color
    Playing(Color),
    /// Analyzing position
    Analyze,
}

/// XBoard engine
pub struct XBoardEngine {
    position: Position,
    tt: TranspositionTable,
    tt_size_mb: usize,
    mode: EngineMode,
    /// Search depth limit (0 = no limit)
    depth_limit: Option<i32>,
    /// Time controls
    time_white: u64,  // milliseconds
    time_black: u64,  // milliseconds
    increment: u64,   // milliseconds per move
    moves_per_tc: u32, // moves per time control (0 = sudden death)
    /// Post thinking output
    post: bool,
    /// Pondering enabled
    ponder: bool,
    /// Game history for draw detection
    game_history: Vec<u64>,
    /// Computer's color
    computer_color: Color,
}

impl XBoardEngine {
    /// Create a new XBoard engine
    pub fn new() -> Self {
        // Initialize magic bitboards
        init_magics();

        XBoardEngine {
            position: Position::new(),
            tt: TranspositionTable::new(64),
            tt_size_mb: 64,
            mode: EngineMode::Force,
            depth_limit: None,
            time_white: 300000,  // 5 minutes default
            time_black: 300000,
            increment: 0,
            moves_per_tc: 0,
            post: true,
            ponder: false,
            game_history: Vec::new(),
            computer_color: Color::Black,
        }
    }

    /// Run the XBoard protocol loop
    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        // XBoard sends "xboard" first, but we might have already consumed it in main
        // Just start accepting commands

        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse command
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            let cmd = tokens[0];

            match cmd {
                "xboard" => {
                    // Already in XBoard mode, just acknowledge
                    writeln!(stdout).unwrap();
                    stdout.flush().unwrap();
                }
                "protover" => self.cmd_protover(&tokens[1..], &mut stdout),
                "accepted" | "rejected" => {
                    // Ignore feature acceptance/rejection
                }
                "new" => self.cmd_new(),
                "quit" => break,
                "force" => self.cmd_force(),
                "go" => self.cmd_go(&mut stdout),
                "playother" => self.cmd_playother(),
                "white" => self.cmd_white(),
                "black" => self.cmd_black(),
                "level" => self.cmd_level(&tokens[1..]),
                "st" => self.cmd_st(&tokens[1..]),
                "sd" => self.cmd_sd(&tokens[1..]),
                "time" => self.cmd_time(&tokens[1..]),
                "otim" => self.cmd_otim(&tokens[1..]),
                "usermove" => self.cmd_usermove(&tokens[1..], &mut stdout),
                "?" => self.cmd_movenow(),
                "ping" => self.cmd_ping(&tokens[1..], &mut stdout),
                "draw" => self.cmd_draw(&mut stdout),
                "result" => self.cmd_result(&tokens[1..]),
                "setboard" => self.cmd_setboard(&tokens[1..]),
                "edit" => self.cmd_edit_mode(&stdin, &mut stdout),
                "hint" => self.cmd_hint(&mut stdout),
                "bk" => self.cmd_bk(&mut stdout),
                "undo" => self.cmd_undo(),
                "remove" => self.cmd_remove(),
                "hard" => self.ponder = true,
                "easy" => self.ponder = false,
                "post" => self.post = true,
                "nopost" => self.post = false,
                "analyze" => self.cmd_analyze(&mut stdout),
                "exit" => self.cmd_exit_analyze(),
                "." => self.cmd_analyze_status(&mut stdout),
                "computer" => {
                    // Opponent is also a computer - we can use this info
                }
                "name" => {
                    // Opponent's name
                }
                "rating" => {
                    // Ratings
                }
                "ics" => {
                    // Playing on ICS
                }
                "memory" => self.cmd_memory(&tokens[1..]),
                "cores" => {
                    // Multi-threading (not implemented yet)
                }
                "egtpath" => {
                    // Endgame tablebase path
                }
                "option" => {
                    // Custom option
                }
                // If it's not a recognized command, try to parse as a move
                _ => {
                    // Try to interpret as a move in coordinate notation
                    if self.try_user_move(cmd, &mut stdout) {
                        // Move was valid and processed
                    } else {
                        writeln!(stdout, "Error (unknown command): {}", cmd).unwrap();
                        stdout.flush().unwrap();
                    }
                }
            }
        }
    }

    /// Handle "protover" command - send feature list
    fn cmd_protover(&self, tokens: &[&str], stdout: &mut io::Stdout) {
        let _version: u32 = tokens.first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        // Send our features
        writeln!(stdout, "feature done=0").unwrap();
        writeln!(stdout, "feature myname=\"Kai 1.0\"").unwrap();
        writeln!(stdout, "feature variants=\"normal\"").unwrap();
        writeln!(stdout, "feature setboard=1").unwrap();
        writeln!(stdout, "feature ping=1").unwrap();
        writeln!(stdout, "feature playother=1").unwrap();
        writeln!(stdout, "feature san=0").unwrap();
        writeln!(stdout, "feature usermove=1").unwrap();
        writeln!(stdout, "feature time=1").unwrap();
        writeln!(stdout, "feature draw=1").unwrap();
        writeln!(stdout, "feature sigint=0").unwrap();
        writeln!(stdout, "feature sigterm=0").unwrap();
        writeln!(stdout, "feature reuse=1").unwrap();
        writeln!(stdout, "feature analyze=1").unwrap();
        writeln!(stdout, "feature colors=0").unwrap();
        writeln!(stdout, "feature ics=0").unwrap();
        writeln!(stdout, "feature name=1").unwrap();
        writeln!(stdout, "feature pause=0").unwrap();
        writeln!(stdout, "feature nps=0").unwrap();
        writeln!(stdout, "feature debug=1").unwrap();
        writeln!(stdout, "feature memory=1").unwrap();
        writeln!(stdout, "feature smp=0").unwrap();
        writeln!(stdout, "feature egt=\"\"").unwrap();
        writeln!(stdout, "feature done=1").unwrap();
        stdout.flush().unwrap();
    }

    /// Handle "new" command - start a new game
    fn cmd_new(&mut self) {
        self.position = Position::new();
        self.tt.clear();
        self.game_history.clear();
        self.mode = EngineMode::Playing(Color::Black);
        self.computer_color = Color::Black;
        self.depth_limit = None;
    }

    /// Handle "force" command - enter force mode
    fn cmd_force(&mut self) {
        self.mode = EngineMode::Force;
        STOP_FLAG.store(true, Ordering::SeqCst);
    }

    /// Handle "go" command - start playing for the side to move
    fn cmd_go(&mut self, stdout: &mut io::Stdout) {
        self.computer_color = self.position.side_to_move;
        self.mode = EngineMode::Playing(self.computer_color);
        self.think_and_move(stdout);
    }

    /// Handle "playother" command - play the color not to move
    fn cmd_playother(&mut self) {
        self.computer_color = self.position.side_to_move.flip();
        self.mode = EngineMode::Playing(self.computer_color);
    }

    /// Handle "white" command (deprecated)
    fn cmd_white(&mut self) {
        self.computer_color = Color::Black;
        self.mode = EngineMode::Playing(Color::Black);
    }

    /// Handle "black" command (deprecated)
    fn cmd_black(&mut self) {
        self.computer_color = Color::White;
        self.mode = EngineMode::Playing(Color::White);
    }

    /// Handle "level" command - set time controls
    fn cmd_level(&mut self, tokens: &[&str]) {
        if tokens.len() < 3 {
            return;
        }

        // level MPS BASE INC
        // MPS = moves per session (0 = sudden death)
        // BASE = base time (can be MIN or MIN:SEC)
        // INC = increment in seconds

        self.moves_per_tc = tokens[0].parse().unwrap_or(0);

        // Parse base time (MIN or MIN:SEC format)
        let base_str = tokens[1];
        let base_ms = if base_str.contains(':') {
            let parts: Vec<&str> = base_str.split(':').collect();
            let mins: u64 = parts[0].parse().unwrap_or(5);
            let secs: u64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            (mins * 60 + secs) * 1000
        } else {
            let mins: u64 = base_str.parse().unwrap_or(5);
            mins * 60 * 1000
        };

        self.time_white = base_ms;
        self.time_black = base_ms;

        // Parse increment (in seconds, convert to milliseconds)
        self.increment = tokens[2].parse::<u64>().unwrap_or(0) * 1000;
    }

    /// Handle "st" command - set time per move
    fn cmd_st(&mut self, tokens: &[&str]) {
        if let Some(secs) = tokens.first().and_then(|s| s.parse::<u64>().ok()) {
            // Set a fixed time per move (store as time limit)
            let time_ms = secs * 1000;
            self.time_white = time_ms;
            self.time_black = time_ms;
            self.moves_per_tc = 1; // One move per time control
        }
    }

    /// Handle "sd" command - set search depth
    fn cmd_sd(&mut self, tokens: &[&str]) {
        if let Some(depth) = tokens.first().and_then(|s| s.parse().ok()) {
            self.depth_limit = Some(depth);
        }
    }

    /// Handle "time" command - set engine's remaining time
    fn cmd_time(&mut self, tokens: &[&str]) {
        if let Some(centisecs) = tokens.first().and_then(|s| s.parse::<u64>().ok()) {
            // Time is in centiseconds
            let time_ms = centisecs * 10;
            match self.computer_color {
                Color::White => self.time_white = time_ms,
                Color::Black => self.time_black = time_ms,
            }
        }
    }

    /// Handle "otim" command - set opponent's remaining time
    fn cmd_otim(&mut self, tokens: &[&str]) {
        if let Some(centisecs) = tokens.first().and_then(|s| s.parse::<u64>().ok()) {
            let time_ms = centisecs * 10;
            match self.computer_color {
                Color::White => self.time_black = time_ms,
                Color::Black => self.time_white = time_ms,
            }
        }
    }

    /// Handle "usermove" command - opponent made a move
    fn cmd_usermove(&mut self, tokens: &[&str], stdout: &mut io::Stdout) {
        if tokens.is_empty() {
            return;
        }

        if !self.try_user_move(tokens[0], stdout) {
            writeln!(stdout, "Illegal move: {}", tokens[0]).unwrap();
            stdout.flush().unwrap();
        }
    }

    /// Try to parse and apply a user move
    fn try_user_move(&mut self, move_str: &str, stdout: &mut io::Stdout) -> bool {
        // Try to parse as coordinate notation (e.g., e2e4, e7e8q)
        if let Some(new_pos) = self.position.make_uci_move(move_str) {
            self.game_history.push(self.position.hash);
            self.position = new_pos;

            // If we're in playing mode and it's our turn, think and move
            if let EngineMode::Playing(color) = self.mode {
                if self.position.side_to_move == color {
                    self.think_and_move(stdout);
                }
            }
            return true;
        }

        // Try SAN notation as fallback
        if let Some(mv) = self.parse_san(move_str) {
            if let Some(new_pos) = self.position.try_make_move(mv) {
                self.game_history.push(self.position.hash);
                self.position = new_pos;

                if let EngineMode::Playing(color) = self.mode {
                    if self.position.side_to_move == color {
                        self.think_and_move(stdout);
                    }
                }
                return true;
            }
        }

        false
    }

    /// Parse SAN (Standard Algebraic Notation) move
    fn parse_san(&self, _san: &str) -> Option<Move> {
        // Basic SAN parsing would go here
        // For now, rely on coordinate notation
        None
    }

    /// Handle "?" command - move immediately
    fn cmd_movenow(&self) {
        STOP_FLAG.store(true, Ordering::SeqCst);
    }

    /// Handle "ping" command - respond with pong
    fn cmd_ping(&self, tokens: &[&str], stdout: &mut io::Stdout) {
        let n = tokens.first().unwrap_or(&"0");
        writeln!(stdout, "pong {}", n).unwrap();
        stdout.flush().unwrap();
    }

    /// Handle "draw" command - offer/accept draw
    fn cmd_draw(&self, stdout: &mut io::Stdout) {
        // Check if position is actually a draw
        if self.is_draw() {
            writeln!(stdout, "offer draw").unwrap();
            stdout.flush().unwrap();
        }
    }

    /// Check if current position is a draw
    fn is_draw(&self) -> bool {
        // 50-move rule
        if self.position.halfmove_clock >= 100 {
            return true;
        }

        // Threefold repetition
        let current_hash = self.position.hash;
        let count = self.game_history.iter().filter(|&&h| h == current_hash).count();
        if count >= 2 {
            return true;
        }

        // Insufficient material check would go here
        false
    }

    /// Handle "result" command - game ended
    fn cmd_result(&mut self, _tokens: &[&str]) {
        self.mode = EngineMode::Force;
        STOP_FLAG.store(true, Ordering::SeqCst);
    }

    /// Handle "setboard" command - set position from FEN
    fn cmd_setboard(&mut self, tokens: &[&str]) {
        let fen = tokens.join(" ");
        if let Ok(pos) = Position::from_fen(&fen) {
            self.position = pos;
            self.game_history.clear();
        }
    }

    /// Handle edit mode (legacy)
    fn cmd_edit_mode(&mut self, stdin: &io::Stdin, stdout: &mut io::Stdout) {
        // Edit mode is complex and mostly unused
        // Just read until "." is received
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if line.trim() == "." {
                break;
            }
        }
        let _ = stdout;
    }

    /// Handle "hint" command - suggest a move
    fn cmd_hint(&mut self, stdout: &mut io::Stdout) {
        // Do a quick search and suggest the best move
        let result = self.position.search(
            &mut self.tt,
            Some(Duration::from_millis(500)),
            Some(6),
            None,
        );
        writeln!(stdout, "Hint: {}", result.best_move.to_uci()).unwrap();
        stdout.flush().unwrap();
    }

    /// Handle "bk" command - show book moves (not implemented)
    fn cmd_bk(&self, stdout: &mut io::Stdout) {
        writeln!(stdout, " No book moves available").unwrap();
        writeln!(stdout).unwrap();
        stdout.flush().unwrap();
    }

    /// Handle "undo" command - undo one move
    fn cmd_undo(&mut self) {
        // We can't really undo without keeping history of positions
        // For now, just note that this should be handled
        if let Some(hash) = self.game_history.pop() {
            // We'd need to store full positions, not just hashes
            let _ = hash;
        }
    }

    /// Handle "remove" command - undo two half-moves
    fn cmd_remove(&mut self) {
        self.game_history.pop();
        self.game_history.pop();
        // Would need position history to properly implement
    }

    /// Handle "analyze" command - enter analysis mode
    fn cmd_analyze(&mut self, stdout: &mut io::Stdout) {
        self.mode = EngineMode::Analyze;
        self.analyze_position(stdout);
    }

    /// Handle "exit" command - exit analysis mode
    fn cmd_exit_analyze(&mut self) {
        self.mode = EngineMode::Force;
        STOP_FLAG.store(true, Ordering::SeqCst);
    }

    /// Handle "." command - show analysis status
    fn cmd_analyze_status(&self, stdout: &mut io::Stdout) {
        writeln!(stdout, "stat01: 0 0 0 0 0").unwrap();
        stdout.flush().unwrap();
    }

    /// Handle "memory" command - set hash table size
    fn cmd_memory(&mut self, tokens: &[&str]) {
        if let Some(size) = tokens.first().and_then(|s| s.parse::<usize>().ok()) {
            let size = size.clamp(1, 4096);
            self.tt_size_mb = size;
            self.tt.resize(size);
        }
    }

    /// Think and make a move
    fn think_and_move(&mut self, stdout: &mut io::Stdout) {
        // Calculate time limit
        let time_limit = self.calculate_time_limit();

        // Reset stop flag
        STOP_FLAG.store(false, Ordering::SeqCst);

        // Run search
        let stop_flag: &'static AtomicBool = unsafe { std::mem::transmute(&STOP_FLAG) };
        let result = self.position.search(
            &mut self.tt,
            time_limit,
            self.depth_limit,
            Some(stop_flag),
        );

        // Output thinking info if post is enabled
        if self.post {
            // XBoard thinking format: ply score time nodes pv
            // time is in centiseconds
            writeln!(
                stdout,
                "{} {} {} {} {}",
                result.depth,
                result.score,
                result.time_ms / 10,
                result.nodes,
                result.pv.iter()
                    .map(|m| m.to_uci())
                    .collect::<Vec<_>>()
                    .join(" ")
            ).unwrap();
        }

        // Check for game end conditions
        if result.score >= 29000 {
            // We're delivering mate
        } else if result.score <= -29000 {
            // We're getting mated
        }

        // Make the move
        let move_str = result.best_move.to_uci();
        self.game_history.push(self.position.hash);
        self.position = self.position.make_move(result.best_move);

        // Output the move
        writeln!(stdout, "move {}", move_str).unwrap();
        stdout.flush().unwrap();

        // Check for draw
        if self.is_draw() {
            writeln!(stdout, "offer draw").unwrap();
            stdout.flush().unwrap();
        }
    }

    /// Calculate time limit for search
    fn calculate_time_limit(&self) -> Option<Duration> {
        let our_time = match self.computer_color {
            Color::White => self.time_white,
            Color::Black => self.time_black,
        };

        if our_time == 0 {
            return None;
        }

        let moves = if self.moves_per_tc > 0 {
            self.moves_per_tc as u64
        } else {
            30 // Estimate for sudden death
        };

        // Simple time management
        let base = our_time / moves.max(1);
        let total = base + (self.increment * 3) / 4;
        let limit = total.min(our_time.saturating_sub(100));

        Some(Duration::from_millis(limit))
    }

    /// Analyze position continuously
    fn analyze_position(&mut self, stdout: &mut io::Stdout) {
        // In analysis mode, we search indefinitely until "exit" or "."/new command
        STOP_FLAG.store(false, Ordering::SeqCst);

        let stop_flag: &'static AtomicBool = unsafe { std::mem::transmute(&STOP_FLAG) };

        // Do iterative deepening, outputting after each depth
        for depth in 1..=100 {
            if STOP_FLAG.load(Ordering::SeqCst) {
                break;
            }

            let result = self.position.search(
                &mut self.tt,
                None,
                Some(depth),
                Some(stop_flag),
            );

            if STOP_FLAG.load(Ordering::SeqCst) {
                break;
            }

            // Output thinking in XBoard format
            writeln!(
                stdout,
                "{} {} {} {} {}",
                result.depth,
                result.score,
                result.time_ms / 10,
                result.nodes,
                result.pv.iter()
                    .map(|m| m.to_uci())
                    .collect::<Vec<_>>()
                    .join(" ")
            ).unwrap();
            stdout.flush().unwrap();
        }
    }
}

impl Default for XBoardEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xboard_engine_creation() {
        let engine = XBoardEngine::new();
        assert_eq!(engine.mode, EngineMode::Force);
    }

    #[test]
    fn test_level_parsing() {
        let mut engine = XBoardEngine::new();

        // Test standard time control: 40 moves in 5 minutes with 0 increment
        engine.cmd_level(&["40", "5", "0"]);
        assert_eq!(engine.moves_per_tc, 40);
        assert_eq!(engine.time_white, 5 * 60 * 1000);
        assert_eq!(engine.increment, 0);

        // Test with MIN:SEC format
        engine.cmd_level(&["0", "2:30", "12"]);
        assert_eq!(engine.moves_per_tc, 0);
        assert_eq!(engine.time_white, (2 * 60 + 30) * 1000);
        assert_eq!(engine.increment, 12 * 1000);
    }

    #[test]
    fn test_new_game() {
        let mut engine = XBoardEngine::new();
        engine.cmd_new();
        assert_eq!(engine.mode, EngineMode::Playing(Color::Black));
        assert_eq!(engine.computer_color, Color::Black);
    }
}

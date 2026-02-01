/// UCI (Universal Chess Interface) protocol implementation
use crate::magic::init_magics;
use crate::position::Position;
use crate::tt::TranspositionTable;
use crate::types::Color;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Global stop flag for search
static STOP_FLAG: AtomicBool = AtomicBool::new(false);

/// UCI engine
pub struct UciEngine {
    position: Position,
    tt: TranspositionTable,
    tt_size_mb: usize,
}

impl UciEngine {
    /// Create a new UCI engine
    pub fn new() -> Self {
        // Initialize magic bitboards
        init_magics();

        UciEngine {
            position: Position::new(),
            tt: TranspositionTable::new(64),
            tt_size_mb: 64,
        }
    }

    /// Run the UCI loop
    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            match tokens[0] {
                "uci" => self.cmd_uci(&mut stdout),
                "isready" => self.cmd_isready(&mut stdout),
                "ucinewgame" => self.cmd_ucinewgame(),
                "position" => self.cmd_position(&tokens[1..]),
                "go" => self.cmd_go(&tokens[1..], &mut stdout),
                "stop" => self.cmd_stop(),
                "quit" => break,
                "setoption" => self.cmd_setoption(&tokens[1..]),

                // Non-standard commands for debugging
                "d" | "display" => self.cmd_display(),
                "perft" => self.cmd_perft(&tokens[1..]),
                "eval" => self.cmd_eval(),

                _ => {}
            }
        }
    }

    /// Handle "uci" command
    fn cmd_uci(&self, stdout: &mut io::Stdout) {
        writeln!(stdout, "id name Kai 1.0").unwrap();
        writeln!(stdout, "id author Sahith Jagarlamudi").unwrap();
        writeln!(stdout).unwrap();
        writeln!(
            stdout,
            "option name Hash type spin default 64 min 1 max 4096"
        )
        .unwrap();
        writeln!(stdout, "uciok").unwrap();
        stdout.flush().unwrap();
    }

    /// Handle "isready" command
    fn cmd_isready(&self, stdout: &mut io::Stdout) {
        writeln!(stdout, "readyok").unwrap();
        stdout.flush().unwrap();
    }

    /// Handle "ucinewgame" command
    fn cmd_ucinewgame(&mut self) {
        self.position = Position::new();
        self.tt.clear();
    }

    /// Handle "position" command
    fn cmd_position(&mut self, tokens: &[&str]) {
        if tokens.is_empty() {
            return;
        }

        let mut idx = 0;

        // Parse position
        if tokens[idx] == "startpos" {
            self.position = Position::new();
            idx += 1;
        } else if tokens[idx] == "fen" {
            idx += 1;
            let fen_parts: Vec<&str> = tokens[idx..]
                .iter()
                .take_while(|&&t| t != "moves")
                .copied()
                .collect();
            let fen = fen_parts.join(" ");
            self.position = Position::from_fen(&fen).unwrap_or_else(|_| Position::new());
            idx += fen_parts.len();
        }

        // Parse moves
        if idx < tokens.len() && tokens[idx] == "moves" {
            idx += 1;
            for move_str in &tokens[idx..] {
                if let Some(new_pos) = self.position.make_uci_move(move_str) {
                    self.position = new_pos;
                }
            }
        }
    }

    /// Handle "go" command
    fn cmd_go(&mut self, tokens: &[&str], stdout: &mut io::Stdout) {
        let mut time_limit = None;
        let mut depth_limit = None;
        let mut wtime = None;
        let mut btime = None;
        let mut winc = None;
        let mut binc = None;
        let mut movestogo = None;
        let mut movetime = None;
        let mut infinite = false;

        let mut i = 0;
        while i < tokens.len() {
            match tokens[i] {
                "depth" => {
                    if i + 1 < tokens.len() {
                        depth_limit = tokens[i + 1].parse().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "movetime" => {
                    if i + 1 < tokens.len() {
                        movetime = tokens[i + 1].parse::<u64>().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "wtime" => {
                    if i + 1 < tokens.len() {
                        wtime = tokens[i + 1].parse::<u64>().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "btime" => {
                    if i + 1 < tokens.len() {
                        btime = tokens[i + 1].parse::<u64>().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "winc" => {
                    if i + 1 < tokens.len() {
                        winc = tokens[i + 1].parse::<u64>().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "binc" => {
                    if i + 1 < tokens.len() {
                        binc = tokens[i + 1].parse::<u64>().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "movestogo" => {
                    if i + 1 < tokens.len() {
                        movestogo = tokens[i + 1].parse::<u32>().ok();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "infinite" => {
                    infinite = true;
                    i += 1;
                }
                "perft" => {
                    if i + 1 < tokens.len() {
                        if let Ok(depth) = tokens[i + 1].parse::<u32>() {
                            self.run_perft(depth, stdout);
                        }
                    }
                    return;
                }
                _ => i += 1,
            }
        }

        // Calculate time limit
        if let Some(mt) = movetime {
            time_limit = Some(Duration::from_millis(mt));
        } else if !infinite {
            let (our_time, our_inc) = match self.position.side_to_move {
                Color::White => (wtime, winc),
                Color::Black => (btime, binc),
            };

            if let Some(time) = our_time {
                let inc = our_inc.unwrap_or(0);
                let moves = movestogo.unwrap_or(30) as u64;

                // Simple time management: use time/moves + most of increment
                let base = time / moves.max(1);
                let total = base + (inc * 3) / 4;

                // Keep some buffer
                let limit = total.min(time.saturating_sub(100));
                time_limit = Some(Duration::from_millis(limit));
            }
        }

        // Reset stop flag
        STOP_FLAG.store(false, Ordering::SeqCst);

        // Run search
        // STOP_FLAG is a static, so &STOP_FLAG already has 'static lifetime - no transmute needed
        let result = self
            .position
            .search(&mut self.tt, time_limit, depth_limit, Some(&STOP_FLAG));

        // Log bestmove for debugging
        eprintln!(
            "BESTMOVE: {} for side {:?}",
            result.best_move.to_uci(),
            self.position.side_to_move
        );
        if let Some(piece) = self.position.piece_at(result.best_move.from_sq()) {
            eprintln!("  Piece at source: {:?}", piece);
        } else {
            eprintln!("  WARNING: No piece at source square!");
        }

        // Output best move
        writeln!(stdout, "bestmove {}", result.best_move.to_uci()).unwrap();
        stdout.flush().unwrap();
    }

    /// Handle "stop" command
    fn cmd_stop(&self) {
        STOP_FLAG.store(true, Ordering::SeqCst);
    }

    /// Handle "setoption" command
    fn cmd_setoption(&mut self, tokens: &[&str]) {
        if tokens.len() < 4 {
            return;
        }

        if tokens[0] != "name" {
            return;
        }

        // Find "value" token
        let value_idx = tokens.iter().position(|&t| t == "value");
        if value_idx.is_none() {
            return;
        }

        let name_parts = &tokens[1..value_idx.unwrap()];
        let name = name_parts.join(" ").to_lowercase();
        let value = tokens[value_idx.unwrap() + 1..].join(" ");

        match name.as_str() {
            "hash" => {
                if let Ok(size) = value.parse::<usize>() {
                    let size = size.clamp(1, 4096);
                    self.tt_size_mb = size;
                    self.tt.resize(size);
                }
            }
            _ => {}
        }
    }

    /// Handle "d" (display) command
    fn cmd_display(&self) {
        self.position.print();
    }

    /// Handle "perft" command
    fn cmd_perft(&self, tokens: &[&str]) {
        if tokens.is_empty() {
            return;
        }

        if let Ok(depth) = tokens[0].parse::<u32>() {
            let mut stdout = io::stdout();
            self.run_perft(depth, &mut stdout);
        }
    }

    /// Run perft with divide output
    fn run_perft(&self, depth: u32, stdout: &mut io::Stdout) {
        use crate::perft::perft_divide;
        use std::time::Instant;

        let start = Instant::now();
        let nodes = perft_divide(&self.position, depth);
        let elapsed = start.elapsed();

        let nps = if elapsed.as_millis() > 0 {
            (nodes as u128 * 1000) / elapsed.as_millis()
        } else {
            0
        };

        writeln!(stdout).unwrap();
        writeln!(stdout, "Nodes: {}", nodes).unwrap();
        writeln!(stdout, "Time: {} ms", elapsed.as_millis()).unwrap();
        writeln!(stdout, "NPS: {}", nps).unwrap();
        stdout.flush().unwrap();
    }

    /// Handle "eval" command
    fn cmd_eval(&self) {
        let score = self.position.evaluate();
        println!("Evaluation: {} cp", score);
    }
}

impl Default for UciEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uci_engine_creation() {
        let _engine = UciEngine::new();
    }

    #[test]
    fn test_position_parsing() {
        let mut engine = UciEngine::new();

        engine.cmd_position(&["startpos"]);
        assert_eq!(engine.position.to_fen(), Position::STARTPOS);

        engine.cmd_position(&["startpos", "moves", "e2e4"]);
        assert_eq!(engine.position.side_to_move, Color::Black);

        engine.cmd_position(&[
            "fen",
            "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R",
            "w",
            "KQkq",
            "-",
            "0",
            "1",
        ]);
        assert!(engine
            .position
            .castling
            .contains(crate::types::CastlingRights::ALL));
    }
}

/// Kai Chess Engine - Main entry point
///
/// Supports both UCI (Universal Chess Interface) and XBoard/WinBoard protocols.
/// The protocol is auto-detected based on the first command received.

mod bitboard;
mod eval;
mod magic;
mod make_move;
mod movegen;
mod moves;
mod ordering;
mod perft;
mod position;
mod qsearch;
mod search;
mod see;
mod tt;
mod types;
mod uci;
mod xboard;
mod zobrist;

use std::io::{self, BufRead, Write};
use uci::UciEngine;
use xboard::XBoardEngine;

/// Protocol type
enum Protocol {
    Uci,
    XBoard,
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Read the first command to detect protocol
    let first_line = {
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            return;
        }
        line
    };

    let first_cmd = first_line.trim();

    // Detect protocol based on first command
    let protocol = if first_cmd == "uci" {
        Protocol::Uci
    } else if first_cmd == "xboard" {
        Protocol::XBoard
    } else {
        // Default to UCI for unrecognized first commands
        // But handle common cases
        match first_cmd {
            // XBoard-style commands
            "protover" | "new" | "force" | "go" | "quit" | "random" | "post" | "nopost"
            | "hard" | "easy" | "ping" | "draw" | "result" | "setboard" | "edit"
            | "hint" | "bk" | "undo" | "remove" | "analyze" | "exit" | "white" | "black"
            | "playother" | "level" | "st" | "sd" | "time" | "otim" | "usermove"
            | "computer" | "name" | "rating" | "ics" | "memory" | "cores" | "egtpath" => {
                Protocol::XBoard
            }
            // UCI-style commands
            "debug" | "isready" | "setoption" | "register" | "ucinewgame" | "position"
            | "stop" | "ponderhit" => {
                Protocol::Uci
            }
            // Unknown - default to UCI but try to handle the command
            _ => Protocol::Uci,
        }
    };

    match protocol {
        Protocol::Uci => {
            let mut engine = UciEngine::new();

            // Handle the first command that was already read
            if first_cmd == "uci" {
                // Process uci command
                writeln!(stdout, "id name Kai 1.0").unwrap();
                writeln!(stdout, "id author Sahith Jagarlamudi").unwrap();
                writeln!(stdout).unwrap();
                writeln!(
                    stdout,
                    "option name Hash type spin default 64 min 1 max 4096"
                ).unwrap();
                writeln!(stdout, "uciok").unwrap();
                stdout.flush().unwrap();
            }

            // Continue with the UCI loop
            engine.run();
        }
        Protocol::XBoard => {
            let mut engine = XBoardEngine::new();

            // For XBoard, we need to handle the first command
            // If it was "xboard", just acknowledge
            if first_cmd == "xboard" {
                writeln!(stdout).unwrap();
                stdout.flush().unwrap();
            } else if first_cmd.starts_with("protover") {
                // Handle protover immediately
                let tokens: Vec<&str> = first_cmd.split_whitespace().collect();
                handle_protover(&tokens[1..], &mut stdout);
            }

            // Continue with the XBoard loop
            engine.run();
        }
    }
}

/// Handle protover command for initial detection
fn handle_protover(_tokens: &[&str], stdout: &mut io::Stdout) {
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

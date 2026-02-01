/// Kai Chess Engine - Main entry point
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
mod zobrist;

use uci::UciEngine;

fn main() {
    let mut engine = UciEngine::new();
    engine.run();
}

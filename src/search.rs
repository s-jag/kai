/// Main search implementation with alpha-beta pruning
use crate::moves::{Move, MoveList};
use crate::ordering::{pick_move, score_moves, SearchHeuristics, MAX_PLY};
use crate::position::Position;
use crate::tt::{Bound, TranspositionTable, TTEntry};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Score constants
pub const INFINITY: i16 = 32000;
pub const MATE_SCORE: i16 = 30000;
pub const MATE_BOUND: i16 = MATE_SCORE - MAX_PLY as i16;

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub best_move: Move,
    pub score: i16,
    pub depth: u8,
    pub nodes: u64,
    pub pv: Vec<Move>,
}

/// Search information and control
pub struct SearchInfo {
    pub nodes: u64,
    pub start_time: Instant,
    pub time_limit: Option<Duration>,
    pub depth_limit: Option<u8>,
    pub stopped: bool,
    pub stop_flag: Option<&'static AtomicBool>,
    pub heuristics: SearchHeuristics,
    pub sel_depth: u8,
}

impl SearchInfo {
    pub fn new(start_time: Instant) -> Self {
        SearchInfo {
            nodes: 0,
            start_time,
            time_limit: None,
            depth_limit: None,
            stopped: false,
            stop_flag: None,
            heuristics: SearchHeuristics::new(),
            sel_depth: 0,
        }
    }

    /// Check if search should stop
    #[inline(always)]
    pub fn should_stop(&mut self) -> bool {
        if self.stopped {
            return true;
        }

        // Check external stop flag
        if let Some(flag) = self.stop_flag {
            if flag.load(Ordering::Relaxed) {
                self.stopped = true;
                return true;
            }
        }

        // Check time limit
        if let Some(limit) = self.time_limit {
            if self.start_time.elapsed() >= limit {
                self.stopped = true;
                return true;
            }
        }

        false
    }
}

/// LMR reduction table
static LMR_TABLE: [[i32; 64]; 64] = init_lmr_table();

const fn init_lmr_table() -> [[i32; 64]; 64] {
    let mut table = [[0i32; 64]; 64];
    let mut depth = 1;
    while depth < 64 {
        let mut moves = 1;
        while moves < 64 {
            // LMR formula from Stockfish
            let ln_depth = ln_approx(depth as f64);
            let ln_moves = ln_approx(moves as f64);
            table[depth][moves] = (0.75 + ln_depth * ln_moves / 2.25) as i32;
            moves += 1;
        }
        depth += 1;
    }
    table
}

/// Approximate natural log for const evaluation
const fn ln_approx(x: f64) -> f64 {
    // Taylor series approximation for ln(x) around x=1
    // Only works for x > 0
    let x = x - 1.0;
    x - (x * x) / 2.0 + (x * x * x) / 3.0 - (x * x * x * x) / 4.0
}

impl Position {
    /// Main search entry point with iterative deepening
    pub fn search(
        &self,
        tt: &mut TranspositionTable,
        time_limit: Option<Duration>,
        depth_limit: Option<u8>,
        stop_flag: Option<&'static AtomicBool>,
    ) -> SearchResult {
        let start_time = Instant::now();
        let mut info = SearchInfo::new(start_time);
        info.time_limit = time_limit;
        info.depth_limit = depth_limit;
        info.stop_flag = stop_flag;

        tt.new_search();

        let max_depth = depth_limit.unwrap_or(MAX_PLY as u8);
        let mut best_move = Move::NULL;
        let mut best_score = -INFINITY;
        let mut pv = Vec::new();

        // Iterative deepening
        for depth in 1..=max_depth {
            let mut alpha = -INFINITY;
            let mut beta = INFINITY;
            let mut delta = 25i16;

            // Aspiration windows after depth 4
            if depth >= 5 {
                alpha = (best_score.saturating_sub(delta)).max(-INFINITY);
                beta = (best_score.saturating_add(delta)).min(INFINITY);
            }

            let mut current_pv = Vec::new();

            loop {
                current_pv.clear();
                let score = self.negamax(
                    depth as i32,
                    0,
                    alpha,
                    beta,
                    &mut info,
                    tt,
                    &mut current_pv,
                    true,
                );

                if info.should_stop() {
                    break;
                }

                // Research if outside aspiration window
                if score <= alpha {
                    beta = (alpha + beta) / 2;
                    alpha = (score.saturating_sub(delta)).max(-INFINITY);
                    delta = delta.saturating_mul(2);
                } else if score >= beta {
                    beta = (score.saturating_add(delta)).min(INFINITY);
                    delta = delta.saturating_mul(2);
                } else {
                    best_score = score;
                    if !current_pv.is_empty() {
                        best_move = current_pv[0];
                        pv = current_pv;
                    }
                    break;
                }
            }

            if info.stopped {
                break;
            }

            // Print UCI info
            let elapsed = start_time.elapsed();
            let nps = if elapsed.as_millis() > 0 {
                (info.nodes as u128 * 1000) / elapsed.as_millis()
            } else {
                0
            };

            print!(
                "info depth {} seldepth {} score {} nodes {} nps {} time {} pv",
                depth,
                info.sel_depth,
                format_score(best_score),
                info.nodes,
                nps,
                elapsed.as_millis()
            );
            for mv in &pv {
                print!(" {}", mv.to_uci());
            }
            println!();

            // If mate found, no need to search deeper
            if best_score.abs() >= MATE_BOUND {
                break;
            }
        }

        SearchResult {
            best_move,
            score: best_score,
            depth: max_depth.min(MAX_PLY as u8),
            nodes: info.nodes,
            pv,
        }
    }

    /// Negamax search with alpha-beta pruning
    fn negamax(
        &self,
        depth: i32,
        ply: i32,
        mut alpha: i16,
        beta: i16,
        info: &mut SearchInfo,
        tt: &mut TranspositionTable,
        pv: &mut Vec<Move>,
        is_pv: bool,
    ) -> i16 {
        // Update selective depth
        if ply as u8 > info.sel_depth {
            info.sel_depth = ply as u8;
        }

        // Check for timeout
        if info.nodes & 2047 == 0 && info.should_stop() {
            return 0;
        }

        info.nodes += 1;

        // Mate distance pruning
        let mating_score = MATE_SCORE - ply as i16;
        if mating_score < beta {
            let mut new_beta = beta;
            if mating_score <= alpha {
                return alpha;
            }
            new_beta = mating_score;
            let _ = new_beta; // Use to avoid warning
        }

        // Check for draw
        if self.halfmove_clock >= 100 {
            return 0;
        }

        let is_root = ply == 0;
        let in_check = self.is_in_check();

        // Probe transposition table
        let tt_entry = tt.probe(self.hash);
        // Validate TT move - must have a piece at source square
        let tt_move = tt_entry
            .map(|e| e.best_move)
            .filter(|mv| mv.is_null() || self.piece_at(mv.from_sq()).is_some())
            .unwrap_or(Move::NULL);

        // TT cutoff (not in PV nodes)
        if !is_pv && !is_root {
            if let Some(entry) = tt_entry {
                if entry.depth_ok(depth) {
                    let score = entry.adjusted_score(ply);
                    match entry.bound {
                        Bound::Exact => return score,
                        Bound::Lower if score >= beta => return score,
                        Bound::Upper if score <= alpha => return score,
                        _ => {}
                    }
                }
            }
        }

        // Drop into quiescence search at depth 0
        if depth <= 0 {
            return self.qsearch(alpha, beta, ply, info, tt);
        }

        // Check extension
        let depth = if in_check { depth + 1 } else { depth };

        // Static evaluation for pruning
        let static_eval = if in_check { -INFINITY } else { self.evaluate() };

        // Reverse futility pruning (static null move pruning)
        if !is_pv && !in_check && depth <= 7 {
            let margin = 80 * depth as i16;
            if static_eval - margin >= beta {
                return static_eval - margin;
            }
        }

        // Null move pruning
        if !is_pv && !in_check && depth >= 3 && static_eval >= beta {
            // Don't do null move if we only have pawns
            let non_pawn_material = (self.piece_bb(self.side_to_move, crate::types::PieceType::Knight)
                | self.piece_bb(self.side_to_move, crate::types::PieceType::Bishop)
                | self.piece_bb(self.side_to_move, crate::types::PieceType::Rook)
                | self.piece_bb(self.side_to_move, crate::types::PieceType::Queen))
            .is_not_empty();

            if non_pawn_material {
                let r = 3 + depth / 4;
                let null_pos = self.make_null_move();
                let score = -null_pos.negamax(
                    depth - 1 - r,
                    ply + 1,
                    -beta,
                    -beta + 1,
                    info,
                    tt,
                    &mut Vec::new(),
                    false,
                );

                if info.stopped {
                    return 0;
                }

                if score >= beta {
                    // Don't return unproven mate scores
                    if score >= MATE_BOUND {
                        return beta;
                    }
                    return score;
                }
            }
        }

        // Generate and order moves
        let mut moves = MoveList::new();
        self.generate_legal_moves(&mut moves);

        // Check for checkmate or stalemate
        if moves.is_empty() {
            return if in_check { -mating_score } else { 0 };
        }

        // Score moves for ordering
        score_moves(&mut moves, self, tt_move, &info.heuristics, ply as usize);

        let mut best_move = Move::NULL;
        let mut best_score = -INFINITY;
        let mut moves_searched = 0;
        let mut local_pv = Vec::new();

        let old_alpha = alpha;

        for i in 0..moves.len() {
            let mv = pick_move(&mut moves, i);
            let new_pos = self.make_move(mv);

            let mut score: i16;

            // Late move reductions
            let reduction = if moves_searched >= 4
                && depth >= 3
                && !mv.is_tactical()
                && !in_check
                && !new_pos.is_in_check()
            {
                let mut r = LMR_TABLE[depth.min(63) as usize][moves_searched.min(63)];
                if !is_pv {
                    r += 1;
                }
                r.min(depth - 1)
            } else {
                0
            };

            // Principal Variation Search
            if moves_searched == 0 {
                // Full window search for first move
                local_pv.clear();
                score = -new_pos.negamax(
                    depth - 1,
                    ply + 1,
                    -beta,
                    -alpha,
                    info,
                    tt,
                    &mut local_pv,
                    is_pv,
                );
            } else {
                // Null window search with LMR
                score = -new_pos.negamax(
                    depth - 1 - reduction,
                    ply + 1,
                    -alpha - 1,
                    -alpha,
                    info,
                    tt,
                    &mut Vec::new(),
                    false,
                );

                // Re-search without reduction if LMR failed high
                if score > alpha && reduction > 0 {
                    score = -new_pos.negamax(
                        depth - 1,
                        ply + 1,
                        -alpha - 1,
                        -alpha,
                        info,
                        tt,
                        &mut Vec::new(),
                        false,
                    );
                }

                // Full re-search if null window search failed high
                if score > alpha && score < beta {
                    local_pv.clear();
                    score = -new_pos.negamax(
                        depth - 1,
                        ply + 1,
                        -beta,
                        -alpha,
                        info,
                        tt,
                        &mut local_pv,
                        true,
                    );
                }
            }

            moves_searched += 1;

            if info.stopped {
                return 0;
            }

            if score > best_score {
                best_score = score;
                best_move = mv;

                if score > alpha {
                    alpha = score;

                    // Update PV
                    pv.clear();
                    pv.push(mv);
                    pv.extend_from_slice(&local_pv);

                    if score >= beta {
                        // Beta cutoff - update heuristics
                        if !mv.is_capture() {
                            info.heuristics.update_killer(mv, ply as usize);
                            info.heuristics
                                .update_history(self.side_to_move, mv, depth, true);
                            info.heuristics
                                .update_countermove(info.heuristics.prev_move, mv);
                        }

                        // Update history for quiet moves that didn't cause cutoff
                        for j in 0..i {
                            let failed_mv = moves.get(j);
                            if !failed_mv.is_capture() {
                                info.heuristics.update_history(
                                    self.side_to_move,
                                    failed_mv,
                                    depth,
                                    false,
                                );
                            }
                        }

                        break;
                    }
                }
            }
        }

        // Store in TT
        let bound = if best_score >= beta {
            Bound::Lower
        } else if best_score > old_alpha {
            Bound::Exact
        } else {
            Bound::Upper
        };

        tt.store(self.hash, depth, best_score, bound, best_move, ply);

        best_score
    }
}

/// Format score for UCI output
fn format_score(score: i16) -> String {
    if score.abs() >= MATE_BOUND {
        let moves_to_mate = if score > 0 {
            (MATE_SCORE - score + 1) / 2
        } else {
            -(MATE_SCORE + score + 1) / 2
        };
        format!("mate {}", moves_to_mate)
    } else {
        format!("cp {}", score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::magic::init_magics;

    fn setup() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            init_magics();
        });
    }

    #[test]
    fn test_search_startpos() {
        setup();
        let pos = Position::new();
        let mut tt = TranspositionTable::new(16);

        let result = pos.search(&mut tt, None, Some(4), None);

        assert!(!result.best_move.is_null());
        assert!(result.nodes > 0);
    }

    #[test]
    fn test_search_finds_mate_in_1() {
        setup();
        // White to move, Qh7#
        let pos = Position::from_fen("k7/8/1K6/8/8/8/8/7Q w - - 0 1").unwrap();
        let mut tt = TranspositionTable::new(16);

        let result = pos.search(&mut tt, None, Some(3), None);

        // Should find mate
        assert!(result.score >= MATE_BOUND);
    }

    #[test]
    fn test_search_finds_forced_mate() {
        setup();
        // Mate in 2
        let pos = Position::from_fen("r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 1").unwrap();
        let mut tt = TranspositionTable::new(16);

        let result = pos.search(&mut tt, None, Some(4), None);

        // Black is getting mated
        assert!(result.score <= -MATE_BOUND);
    }

    #[test]
    fn test_search_with_time_limit() {
        setup();
        let pos = Position::new();
        let mut tt = TranspositionTable::new(16);

        let result = pos.search(&mut tt, Some(Duration::from_millis(100)), None, None);

        assert!(!result.best_move.is_null());
    }
}

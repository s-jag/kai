/// Move ordering for search
use crate::moves::{Move, MoveList, MAX_MOVES};
use crate::position::Position;
use crate::see::see_piece_value;
use crate::types::{Color, PieceType, Square};

/// Move ordering scores
const TT_MOVE_SCORE: i32 = 10_000_000;
const GOOD_CAPTURE_BASE: i32 = 8_000_000;
const KILLER_SCORE_1: i32 = 6_000_000;
const KILLER_SCORE_2: i32 = 5_000_000;
const COUNTER_MOVE_SCORE: i32 = 4_000_000;
const BAD_CAPTURE_BASE: i32 = -2_000_000;

/// Maximum ply for killer/history storage
pub const MAX_PLY: usize = 128;

/// Search heuristics for move ordering
pub struct SearchHeuristics {
    /// Killer moves (2 per ply)
    pub killers: [[Move; 2]; MAX_PLY],

    /// History heuristic [color][from][to]
    pub history: [[[i32; 64]; 64]; 2],

    /// Counter move heuristic [from][to]
    pub countermoves: [[Move; 64]; 64],

    /// Previous move for countermove lookup
    pub prev_move: Move,
}

impl SearchHeuristics {
    pub fn new() -> Self {
        SearchHeuristics {
            killers: [[Move::NULL; 2]; MAX_PLY],
            history: [[[0; 64]; 64]; 2],
            countermoves: [[Move::NULL; 64]; 64],
            prev_move: Move::NULL,
        }
    }

    /// Clear all heuristics
    pub fn clear(&mut self) {
        self.killers = [[Move::NULL; 2]; MAX_PLY];
        self.history = [[[0; 64]; 64]; 2];
        self.countermoves = [[Move::NULL; 64]; 64];
        self.prev_move = Move::NULL;
    }

    /// Update killer moves
    pub fn update_killer(&mut self, mv: Move, ply: usize) {
        if ply >= MAX_PLY {
            return;
        }
        // Don't add captures as killers
        if mv.is_capture() {
            return;
        }
        // Don't add if already the first killer
        if self.killers[ply][0] == mv {
            return;
        }
        // Shift killers
        self.killers[ply][1] = self.killers[ply][0];
        self.killers[ply][0] = mv;
    }

    /// Update history heuristic
    pub fn update_history(&mut self, color: Color, mv: Move, depth: i32, is_good: bool) {
        if mv.is_capture() || mv.is_promotion() {
            return;
        }

        let from = mv.from_sq().0 as usize;
        let to = mv.to_sq().0 as usize;
        let bonus = if is_good { depth * depth } else { -(depth * depth) };

        // Gravity formula to prevent overflow
        let history = &mut self.history[color as usize][from][to];
        *history += bonus - (*history * bonus.abs() / 16384);
    }

    /// Update countermove heuristic
    pub fn update_countermove(&mut self, prev_move: Move, mv: Move) {
        if prev_move.is_null() || mv.is_capture() {
            return;
        }
        let from = prev_move.from_sq().0 as usize;
        let to = prev_move.to_sq().0 as usize;
        self.countermoves[from][to] = mv;
    }

    /// Get history score for a move
    pub fn get_history(&self, color: Color, mv: Move) -> i32 {
        let from = mv.from_sq().0 as usize;
        let to = mv.to_sq().0 as usize;
        self.history[color as usize][from][to]
    }

    /// Check if move is a killer
    pub fn is_killer(&self, mv: Move, ply: usize) -> Option<u8> {
        if ply >= MAX_PLY {
            return None;
        }
        if self.killers[ply][0] == mv {
            Some(0)
        } else if self.killers[ply][1] == mv {
            Some(1)
        } else {
            None
        }
    }

    /// Check if move is the countermove
    pub fn is_countermove(&self, prev_move: Move, mv: Move) -> bool {
        if prev_move.is_null() {
            return false;
        }
        let from = prev_move.from_sq().0 as usize;
        let to = prev_move.to_sq().0 as usize;
        self.countermoves[from][to] == mv
    }
}

impl Default for SearchHeuristics {
    fn default() -> Self {
        Self::new()
    }
}

/// MVV-LVA table [victim][attacker]
/// Higher score = better capture (capture valuable piece with less valuable piece)
const MVV_LVA: [[i32; 6]; 6] = [
    // Pawn victim
    [105, 104, 103, 102, 101, 100],
    // Knight victim
    [205, 204, 203, 202, 201, 200],
    // Bishop victim
    [305, 304, 303, 302, 301, 300],
    // Rook victim
    [405, 404, 403, 402, 401, 400],
    // Queen victim
    [505, 504, 503, 502, 501, 500],
    // King victim (shouldn't happen)
    [605, 604, 603, 602, 601, 600],
];

/// Score moves for ordering
pub fn score_moves(
    list: &mut MoveList,
    pos: &Position,
    tt_move: Move,
    heuristics: &SearchHeuristics,
    ply: usize,
) {
    for i in 0..list.len() {
        let mv = list.get(i);
        let score = score_move(pos, mv, tt_move, heuristics, ply);
        list.set_score(i, score);
    }
}

/// Score a single move
fn score_move(
    pos: &Position,
    mv: Move,
    tt_move: Move,
    heuristics: &SearchHeuristics,
    ply: usize,
) -> i32 {
    // TT move gets highest priority
    if mv == tt_move {
        return TT_MOVE_SCORE;
    }

    // Captures use MVV-LVA or SEE
    if mv.is_capture() {
        let victim = if mv.is_en_passant() {
            PieceType::Pawn
        } else {
            match pos.piece_at(mv.to_sq()) {
                Some(p) => p.piece_type(),
                None => return 0, // Invalid move, give it lowest priority
            }
        };
        let attacker = match pos.piece_at(mv.from_sq()) {
            Some(p) => p.piece_type(),
            None => return 0, // Invalid move, give it lowest priority
        };

        let mvv_lva = MVV_LVA[victim as usize][attacker as usize];

        // Use SEE to classify as good or bad capture
        if pos.see_ge(mv, 0) {
            return GOOD_CAPTURE_BASE + mvv_lva;
        } else {
            return BAD_CAPTURE_BASE + mvv_lva;
        }
    }

    // Promotions
    if mv.is_promotion() {
        let promo_value = see_piece_value(mv.promotion_piece());
        return GOOD_CAPTURE_BASE + promo_value as i32;
    }

    // Killer moves
    if let Some(killer_idx) = heuristics.is_killer(mv, ply) {
        return if killer_idx == 0 {
            KILLER_SCORE_1
        } else {
            KILLER_SCORE_2
        };
    }

    // Countermove
    if heuristics.is_countermove(heuristics.prev_move, mv) {
        return COUNTER_MOVE_SCORE;
    }

    // History heuristic
    heuristics.get_history(pos.side_to_move, mv)
}

/// Pick the best move from the remaining moves (selection sort)
/// Moves the best move to position `start` and returns it
pub fn pick_move(list: &mut MoveList, start: usize) -> Move {
    let mut best_idx = start;
    let mut best_score = list.score(start);

    for i in (start + 1)..list.len() {
        if list.score(i) > best_score {
            best_score = list.score(i);
            best_idx = i;
        }
    }

    if best_idx != start {
        list.swap(start, best_idx);
    }

    list.get(start)
}

/// Score captures only (for quiescence search)
pub fn score_captures(list: &mut MoveList, pos: &Position) {
    for i in 0..list.len() {
        let mv = list.get(i);
        let score = score_capture(pos, mv);
        list.set_score(i, score);
    }
}

/// Score a capture move
fn score_capture(pos: &Position, mv: Move) -> i32 {
    if mv.is_capture() {
        let victim = if mv.is_en_passant() {
            PieceType::Pawn
        } else {
            pos.piece_at(mv.to_sq()).expect("Capture but no piece").piece_type()
        };
        let attacker = pos.piece_at(mv.from_sq()).expect("No piece at source").piece_type();

        MVV_LVA[victim as usize][attacker as usize]
    } else if mv.is_promotion() {
        // Treat promotions as valuable captures
        see_piece_value(mv.promotion_piece()) as i32
    } else {
        0
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
    fn test_killer_moves() {
        let mut h = SearchHeuristics::new();
        let mv1 = Move::quiet(Square::E2, Square::E4);
        let mv2 = Move::quiet(Square::D2, Square::D4);

        h.update_killer(mv1, 0);
        assert_eq!(h.killers[0][0], mv1);

        h.update_killer(mv2, 0);
        assert_eq!(h.killers[0][0], mv2);
        assert_eq!(h.killers[0][1], mv1);

        assert_eq!(h.is_killer(mv2, 0), Some(0));
        assert_eq!(h.is_killer(mv1, 0), Some(1));
    }

    #[test]
    fn test_history_update() {
        let mut h = SearchHeuristics::new();
        let mv = Move::quiet(Square::E2, Square::E4);

        h.update_history(Color::White, mv, 5, true);
        assert!(h.get_history(Color::White, mv) > 0);

        h.update_history(Color::White, mv, 5, false);
        // Should decrease but formula prevents going too negative
    }

    #[test]
    fn test_mvv_lva() {
        setup();
        // Queen taking pawn should be scored lower than pawn taking queen
        let pxq = MVV_LVA[PieceType::Queen as usize][PieceType::Pawn as usize];
        let qxp = MVV_LVA[PieceType::Pawn as usize][PieceType::Queen as usize];
        assert!(pxq > qxp);
    }

    #[test]
    fn test_move_scoring() {
        setup();
        let pos = Position::new();
        let mut list = MoveList::new();
        pos.generate_legal_moves(&mut list);

        let heuristics = SearchHeuristics::new();
        score_moves(&mut list, &pos, Move::NULL, &heuristics, 0);

        // Without TT move or killers, all quiet moves should have history scores (0)
        for i in 0..list.len() {
            // All startpos moves are quiet, so should have low scores
            assert!(list.score(i) <= 0);
        }
    }
}

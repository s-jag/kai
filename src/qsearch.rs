/// Quiescence search - search only captures to reach a quiet position
use crate::moves::MoveList;
use crate::ordering::{pick_move, score_captures};
use crate::position::Position;
use crate::search::SearchInfo;
use crate::tt::TranspositionTable;

/// Maximum quiescence depth
const MAX_QSEARCH_DEPTH: i32 = 10;

/// Delta pruning margin (queen value)
const DELTA_MARGIN: i16 = 900;

impl Position {
    /// Quiescence search
    /// qs_ply tracks depth within quiescence search (starts at 0)
    pub fn qsearch(
        &self,
        mut alpha: i16,
        beta: i16,
        qs_ply: i32,
        info: &mut SearchInfo,
        _tt: &mut TranspositionTable,
    ) -> i16 {
        info.nodes += 1;

        // Check for timeout periodically
        if info.nodes & 2047 == 0 && info.should_stop() {
            return 0;
        }

        // Stand pat evaluation
        let stand_pat = self.evaluate();

        // Beta cutoff
        if stand_pat >= beta {
            return stand_pat;
        }

        // Delta pruning - if we can't possibly raise alpha, return early
        if stand_pat + DELTA_MARGIN < alpha {
            return alpha;
        }

        // Update alpha
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        // Limit quiescence depth - qs_ply is the depth within qsearch (0 at entry)
        if qs_ply >= MAX_QSEARCH_DEPTH {
            return stand_pat;
        }

        // Generate and score captures
        let mut moves = MoveList::new();
        self.generate_captures(&mut moves);
        score_captures(&mut moves, self);

        // Search captures
        for i in 0..moves.len() {
            let mv = pick_move(&mut moves, i);

            // SEE pruning - skip clearly losing captures
            if !self.see_ge(mv, 0) {
                continue;
            }

            // Delta pruning for individual captures
            if !mv.is_promotion() {
                let captured_value = if mv.is_en_passant() {
                    100 // Pawn value
                } else {
                    match self.piece_at(mv.to_sq()) {
                        Some(p) => crate::see::see_piece_value(p.piece_type()),
                        None => continue, // Invalid capture, skip
                    }
                };

                if stand_pat + captured_value + 200 < alpha {
                    continue;
                }
            }

            // Skip illegal moves (generate_captures produces pseudo-legal moves)
            if !self.is_legal(mv) {
                continue;
            }

            // Make move and recurse
            let new_pos = self.make_move(mv);
            let score = -new_pos.qsearch(-beta, -alpha, qs_ply + 1, info, _tt);

            // Check for timeout
            if info.stopped {
                return 0;
            }

            // Beta cutoff
            if score >= beta {
                return score;
            }

            // Update alpha
            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::magic::init_magics;
    use crate::search::SearchInfo;
    use crate::tt::TranspositionTable;
    use std::time::Instant;

    fn setup() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            init_magics();
        });
    }

    #[test]
    fn test_qsearch_quiet_position() {
        setup();
        let pos = Position::new();
        let mut info = SearchInfo::new(Instant::now());
        let mut tt = TranspositionTable::new(1);

        let score = pos.qsearch(-30000, 30000, 0, &mut info, &mut tt);

        // Starting position has no captures, should return stand pat
        assert!(score.abs() < 50);
    }

    #[test]
    fn test_qsearch_winning_capture() {
        setup();
        // White can capture a free queen
        let pos =
            Position::from_fen("4k3/8/4q3/8/8/4R3/8/4K3 w - - 0 1").unwrap();
        let mut info = SearchInfo::new(Instant::now());
        let mut tt = TranspositionTable::new(1);

        let score = pos.qsearch(-30000, 30000, 0, &mut info, &mut tt);

        // Should find the queen capture
        assert!(score > 800, "Should find winning capture: {}", score);
    }

    #[test]
    fn test_qsearch_nodes() {
        setup();
        let pos = Position::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        )
        .unwrap();
        let mut info = SearchInfo::new(Instant::now());
        let mut tt = TranspositionTable::new(1);

        let _ = pos.qsearch(-30000, 30000, 0, &mut info, &mut tt);

        // Should search some nodes
        assert!(info.nodes > 0);
    }
}

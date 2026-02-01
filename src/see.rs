/// Static Exchange Evaluation (SEE)
use crate::bitboard::{king_attacks, knight_attacks, pawn_attacks, Bitboard};
use crate::magic::{bishop_attacks, rook_attacks};
use crate::moves::Move;
use crate::position::Position;
use crate::types::{Color, PieceType};

/// Piece values for SEE (simpler than eval values)
const SEE_VALUES: [i16; 6] = [
    100,  // Pawn
    300,  // Knight
    300,  // Bishop
    500,  // Rook
    900,  // Queen
    10000, // King (should never be captured)
];

impl Position {
    /// Check if SEE of a move is >= threshold
    /// Returns true if the move is winning or equal according to SEE
    pub fn see_ge(&self, mv: Move, threshold: i16) -> bool {
        let from = mv.from_sq();
        let to = mv.to_sq();

        // Get value of captured piece (if any)
        let mut value = if mv.is_capture() {
            if mv.is_en_passant() {
                SEE_VALUES[PieceType::Pawn as usize]
            } else {
                match self.piece_at(to) {
                    Some(captured) => SEE_VALUES[captured.piece_type() as usize],
                    None => return false, // Invalid capture, assume losing
                }
            }
        } else {
            0
        };

        // Handle promotion
        if mv.is_promotion() {
            let promo_type = mv.promotion_piece();
            value += SEE_VALUES[promo_type as usize] - SEE_VALUES[PieceType::Pawn as usize];
        }

        // Get the attacking piece value
        let attacker = match self.piece_at(from) {
            Some(p) => p,
            None => return false, // No piece at source, invalid move
        };
        let attacker_value = SEE_VALUES[attacker.piece_type() as usize];

        // Quick check: if we're capturing something and can afford to lose the attacker,
        // the exchange is winning
        if value - attacker_value >= threshold {
            return true;
        }

        // Build occupancy without the moving piece
        let mut occupied = self.all_occupied.clear(from);

        // Handle en passant: remove the captured pawn
        if mv.is_en_passant() {
            let us = self.side_to_move;
            let captured_sq = crate::types::Square(
                (to.0 as i8 + if us == Color::White { -8 } else { 8 }) as u8,
            );
            occupied = occupied.clear(captured_sq);
        }

        // Get all attackers to the target square
        let mut attackers = self.attackers_to(to, occupied) & occupied;

        // Start the exchange
        let mut side_to_move = self.side_to_move.flip();
        let mut gain = [0i16; 32];
        let mut depth = 0;

        gain[0] = value;
        let mut piece_on_sq = attacker.piece_type();

        loop {
            depth += 1;
            gain[depth] = SEE_VALUES[piece_on_sq as usize] - gain[depth - 1];

            // Pruning: if the current side can't improve even with a max gain, exit
            if (-gain[depth - 1]).max(gain[depth]) < threshold {
                break;
            }

            // Find least valuable attacker for the side to move
            let stm_attackers = attackers & self.occupied[side_to_move as usize];
            if stm_attackers.is_empty() {
                break;
            }

            // Find LVA (Least Valuable Attacker)
            let (attacker_sq, attacker_type) = self.find_lva(stm_attackers);

            // Remove the attacker from occupied
            occupied = occupied.clear(attacker_sq);
            attackers = attackers.clear(attacker_sq);

            // Update x-ray attackers (sliders behind the attacker)
            if attacker_type == PieceType::Pawn
                || attacker_type == PieceType::Bishop
                || attacker_type == PieceType::Queen
            {
                attackers |=
                    bishop_attacks(to, occupied) & self.diagonal_sliders_all() & occupied;
            }
            if attacker_type == PieceType::Rook || attacker_type == PieceType::Queen {
                attackers |=
                    rook_attacks(to, occupied) & self.orthogonal_sliders_all() & occupied;
            }

            piece_on_sq = attacker_type;
            side_to_move = side_to_move.flip();
        }

        // Negamax the gain array
        while depth > 1 {
            depth -= 1;
            gain[depth - 1] = -(-gain[depth - 1]).max(gain[depth]);
        }

        gain[0] >= threshold
    }

    /// Get the SEE value of a capture move
    pub fn see_value(&self, mv: Move) -> i16 {
        // Find the actual SEE value through binary search
        let mut lo = -1500i16;
        let mut hi = 1500i16;

        while lo < hi {
            let mid = (lo + hi + 1) / 2;
            if self.see_ge(mv, mid) {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }

        lo
    }

    /// Find the least valuable attacker in a set of attackers
    fn find_lva(&self, attackers: Bitboard) -> (crate::types::Square, PieceType) {
        for pt in [
            PieceType::Pawn,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Rook,
            PieceType::Queen,
            PieceType::King,
        ] {
            let piece_bb = (self.piece_bb(Color::White, pt) | self.piece_bb(Color::Black, pt))
                & attackers;
            if piece_bb.is_not_empty() {
                return (piece_bb.lsb(), pt);
            }
        }
        unreachable!("No attacker found");
    }

    /// Get all diagonal sliders (both colors)
    fn diagonal_sliders_all(&self) -> Bitboard {
        self.diagonal_sliders(Color::White) | self.diagonal_sliders(Color::Black)
    }

    /// Get all orthogonal sliders (both colors)
    fn orthogonal_sliders_all(&self) -> Bitboard {
        self.orthogonal_sliders(Color::White) | self.orthogonal_sliders(Color::Black)
    }
}

/// Get piece value for SEE
pub fn see_piece_value(pt: PieceType) -> i16 {
    SEE_VALUES[pt as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::magic::init_magics;
    use crate::types::Square;

    fn setup() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            init_magics();
        });
    }

    #[test]
    fn test_see_winning_capture() {
        setup();
        // White queen takes undefended pawn
        let pos =
            Position::from_fen("4k3/8/4p3/8/8/4Q3/8/4K3 w - - 0 1").unwrap();
        let mv = Move::capture(Square::from_algebraic("e3").unwrap(), Square::from_algebraic("e6").unwrap());
        assert!(pos.see_ge(mv, 0), "Queen takes pawn should be winning");
        assert!(pos.see_ge(mv, 100), "Should win at least a pawn");
    }

    #[test]
    fn test_see_losing_capture() {
        setup();
        // White queen takes defended pawn
        let pos =
            Position::from_fen("4k3/4r3/4p3/8/8/4Q3/8/4K3 w - - 0 1").unwrap();
        let mv = Move::capture(Square::from_algebraic("e3").unwrap(), Square::from_algebraic("e6").unwrap());
        assert!(!pos.see_ge(mv, 0), "Queen takes defended pawn should be losing");
    }

    #[test]
    fn test_see_equal_exchange() {
        setup();
        // Knight takes knight
        let pos =
            Position::from_fen("4k3/8/4n3/8/8/4N3/8/4K3 w - - 0 1").unwrap();
        let mv = Move::capture(Square::from_algebraic("e3").unwrap(), Square::from_algebraic("e6").unwrap());
        assert!(pos.see_ge(mv, 0), "Knight takes knight should be equal");
        assert!(!pos.see_ge(mv, 100), "Should not win material");
    }

    #[test]
    fn test_see_complex_exchange() {
        setup();
        // Pawn takes knight, knight retakes, pawn retakes
        let pos = Position::from_fen("4k3/8/3n4/4n3/3P4/4P3/8/4K3 w - - 0 1").unwrap();
        let mv = Move::capture(Square::from_algebraic("e3").unwrap(), Square::from_algebraic("d4").unwrap());
        // This is a bad capture - pawn takes nothing, knight takes pawn
        assert!(!pos.see_ge(mv, 0));
    }

    #[test]
    fn test_see_xray() {
        setup();
        // Rook takes rook, but there's another rook behind
        let pos =
            Position::from_fen("3rk3/8/8/8/8/8/8/R2RK3 w - - 0 1").unwrap();
        let mv = Move::capture(Square::from_algebraic("d1").unwrap(), Square::from_algebraic("d8").unwrap());
        assert!(pos.see_ge(mv, 0), "RxR with x-ray should be winning");
    }
}

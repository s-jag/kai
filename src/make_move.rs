/// Make move implementation (copy-make approach)
use crate::moves::Move;
use crate::position::Position;
use crate::types::{CastlingRights, Color, Piece, PieceType, Square};
use crate::zobrist::ZOBRIST;

/// Castling rights update table - indexed by square
/// When a piece moves from or to a square, AND with this mask
const CASTLING_RIGHTS_UPDATE: [u8; 64] = {
    let mut table = [0x0Fu8; 64]; // All rights preserved by default

    // White pieces
    table[Square::A1.0 as usize] = 0x0D; // Remove white queenside
    table[Square::E1.0 as usize] = 0x0C; // Remove both white
    table[Square::H1.0 as usize] = 0x0E; // Remove white kingside

    // Black pieces
    table[Square::A8.0 as usize] = 0x07; // Remove black queenside
    table[Square::E8.0 as usize] = 0x03; // Remove both black
    table[Square::H8.0 as usize] = 0x0B; // Remove black kingside

    table
};

impl Position {
    /// Make a move and return the new position (copy-make approach)
    pub fn make_move(&self, mv: Move) -> Self {
        let mut new = self.clone();
        new.apply_move(mv);
        new
    }

    /// Apply a move to the position (modifies in place)
    fn apply_move(&mut self, mv: Move) {
        let us = self.side_to_move;
        let them = us.flip();
        let from = mv.from_sq();
        let to = mv.to_sq();
        let flags = mv.flags();

        // Get the moving piece
        let piece = self.board[from.0 as usize].expect("No piece at source square");
        let piece_type = piece.piece_type();

        // Update en passant (remove old EP square from hash)
        if let Some(ep_sq) = self.en_passant {
            self.hash ^= ZOBRIST.en_passant_key(ep_sq.file());
        }
        self.en_passant = None;

        // Handle captures
        let captured = if mv.is_en_passant() {
            // En passant capture - captured pawn is not on destination square
            let captured_sq = Square((to.0 as i8 + if us == Color::White { -8 } else { 8 }) as u8);
            let captured_piece = self.board[captured_sq.0 as usize];
            self.remove_piece_internal(captured_sq, them, PieceType::Pawn);
            captured_piece
        } else if mv.is_capture() {
            // Normal capture
            let captured_piece = self.board[to.0 as usize];
            if let Some(cp) = captured_piece {
                self.remove_piece_internal(to, them, cp.piece_type());
            }
            captured_piece
        } else {
            None
        };

        // Move the piece
        self.remove_piece_internal(from, us, piece_type);

        // Handle promotions
        let final_piece_type = if mv.is_promotion() {
            mv.promotion_piece()
        } else {
            piece_type
        };

        self.put_piece_internal(to, us, final_piece_type);

        // Handle castling
        if mv.is_castle() {
            let (rook_from, rook_to) = if mv.is_kingside_castle() {
                match us {
                    Color::White => (Square::H1, Square::F1),
                    Color::Black => (Square::H8, Square::F8),
                }
            } else {
                match us {
                    Color::White => (Square::A1, Square::D1),
                    Color::Black => (Square::A8, Square::D8),
                }
            };

            self.remove_piece_internal(rook_from, us, PieceType::Rook);
            self.put_piece_internal(rook_to, us, PieceType::Rook);
        }

        // Update castling rights
        let old_castling = self.castling;
        self.castling = CastlingRights(
            self.castling.0
                & CASTLING_RIGHTS_UPDATE[from.0 as usize]
                & CASTLING_RIGHTS_UPDATE[to.0 as usize],
        );
        if self.castling != old_castling {
            self.hash ^= ZOBRIST.castling_key(old_castling);
            self.hash ^= ZOBRIST.castling_key(self.castling);
        }

        // Set en passant square for double pawn pushes
        if mv.is_double_push() {
            let ep_sq = Square((from.0 as i8 + if us == Color::White { 8 } else { -8 }) as u8);
            self.en_passant = Some(ep_sq);
            self.hash ^= ZOBRIST.en_passant_key(ep_sq.file());
        }

        // Update halfmove clock
        if mv.is_capture() || piece_type == PieceType::Pawn {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        // Update fullmove number
        if us == Color::Black {
            self.fullmove_number += 1;
        }

        // Switch side to move
        self.side_to_move = them;
        self.hash ^= ZOBRIST.side_key();

        // Update checkers
        self.checkers = self.compute_checkers();
    }

    /// Make a null move (pass) - for null move pruning
    pub fn make_null_move(&self) -> Self {
        let mut new = self.clone();

        // Remove en passant from hash
        if let Some(ep_sq) = new.en_passant {
            new.hash ^= ZOBRIST.en_passant_key(ep_sq.file());
        }
        new.en_passant = None;

        // Switch side
        new.side_to_move = new.side_to_move.flip();
        new.hash ^= ZOBRIST.side_key();

        // Update checkers (should be empty after null move if legal)
        new.checkers = new.compute_checkers();

        new
    }

    /// Internal helper to remove a piece and update hash
    fn remove_piece_internal(&mut self, sq: Square, color: Color, piece_type: PieceType) {
        self.pieces[color as usize][piece_type as usize] =
            self.pieces[color as usize][piece_type as usize].clear(sq);
        self.occupied[color as usize] = self.occupied[color as usize].clear(sq);
        self.all_occupied = self.all_occupied.clear(sq);
        self.board[sq.0 as usize] = None;

        // Update hash
        self.hash ^= ZOBRIST.piece_key(color, piece_type, sq);
    }

    /// Internal helper to put a piece and update hash
    fn put_piece_internal(&mut self, sq: Square, color: Color, piece_type: PieceType) {
        self.pieces[color as usize][piece_type as usize] =
            self.pieces[color as usize][piece_type as usize].set(sq);
        self.occupied[color as usize] = self.occupied[color as usize].set(sq);
        self.all_occupied = self.all_occupied.set(sq);
        self.board[sq.0 as usize] = Some(Piece::new(color, piece_type));

        // Update king position cache
        if piece_type == PieceType::King {
            self.king_sq[color as usize] = sq;
        }

        // Update hash
        self.hash ^= ZOBRIST.piece_key(color, piece_type, sq);
    }

    /// Parse and make a move from UCI notation
    pub fn make_uci_move(&self, uci: &str) -> Option<Self> {
        let mv = self.parse_uci_move(uci)?;
        Some(self.make_move(mv))
    }

    /// Parse a UCI move string
    pub fn parse_uci_move(&self, uci: &str) -> Option<Move> {
        if uci.len() < 4 {
            return None;
        }

        let from = Square::from_algebraic(&uci[0..2])?;
        let to = Square::from_algebraic(&uci[2..4])?;
        let promo = uci.chars().nth(4);

        // Generate legal moves and find match
        let mut list = crate::moves::MoveList::new();
        self.generate_legal_moves(&mut list);

        for mv in list.iter() {
            if mv.from_sq() == from && mv.to_sq() == to {
                // Check promotion match
                if mv.is_promotion() {
                    let promo_char = promo?;
                    let expected = match mv.promotion_piece() {
                        PieceType::Knight => 'n',
                        PieceType::Bishop => 'b',
                        PieceType::Rook => 'r',
                        PieceType::Queen => 'q',
                        _ => continue,
                    };
                    if promo_char.to_ascii_lowercase() != expected {
                        continue;
                    }
                }
                return Some(mv);
            }
        }

        None
    }
}

// Square constants needed for castling
impl Square {
    pub const F1: Square = Square(5);
    pub const F8: Square = Square(61);
    pub const D1: Square = Square(3);
    pub const D8: Square = Square(59);
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
    fn test_make_move_pawn() {
        setup();
        let pos = Position::new();
        let new_pos = pos.make_uci_move("e2e4").unwrap();

        assert_eq!(new_pos.side_to_move, Color::Black);
        assert!(new_pos.en_passant.is_some());
        assert_eq!(new_pos.en_passant.unwrap(), Square::from_algebraic("e3").unwrap());
    }

    #[test]
    fn test_make_move_capture() {
        setup();
        let pos = Position::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2")
            .unwrap();
        let new_pos = pos.make_uci_move("e4d5").unwrap();

        // Check the pawn is on d5
        assert!(new_pos
            .piece_bb(Color::White, PieceType::Pawn)
            .contains(Square::from_algebraic("d5").unwrap()));
        // Check black pawn is gone
        assert!(!new_pos
            .piece_bb(Color::Black, PieceType::Pawn)
            .contains(Square::from_algebraic("d5").unwrap()));
    }

    #[test]
    fn test_make_move_castling_kingside() {
        setup();
        let pos =
            Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        let new_pos = pos.make_uci_move("e1g1").unwrap();

        // King should be on g1
        assert_eq!(new_pos.king_sq[Color::White as usize], Square::G1);
        // Rook should be on f1
        assert!(new_pos
            .piece_bb(Color::White, PieceType::Rook)
            .contains(Square::F1));
        // Castling rights should be updated
        assert!(!new_pos.castling.contains(CastlingRights::WHITE_KINGSIDE));
        assert!(!new_pos.castling.contains(CastlingRights::WHITE_QUEENSIDE));
    }

    #[test]
    fn test_make_move_castling_queenside() {
        setup();
        let pos =
            Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        let new_pos = pos.make_uci_move("e1c1").unwrap();

        // King should be on c1
        assert_eq!(new_pos.king_sq[Color::White as usize], Square::C1);
        // Rook should be on d1
        assert!(new_pos
            .piece_bb(Color::White, PieceType::Rook)
            .contains(Square::D1));
    }

    #[test]
    fn test_make_move_en_passant() {
        setup();
        let pos =
            Position::from_fen("rnbqkbnr/pppp1ppp/8/4pP2/8/8/PPPPP1PP/RNBQKBNR w KQkq e6 0 1")
                .unwrap();
        let new_pos = pos.make_uci_move("f5e6").unwrap();

        // White pawn should be on e6
        assert!(new_pos
            .piece_bb(Color::White, PieceType::Pawn)
            .contains(Square::from_algebraic("e6").unwrap()));
        // Black pawn on e5 should be gone
        assert!(!new_pos
            .piece_bb(Color::Black, PieceType::Pawn)
            .contains(Square::from_algebraic("e5").unwrap()));
    }

    #[test]
    fn test_make_move_promotion() {
        setup();
        let pos = Position::from_fen("8/P7/8/8/8/8/8/4K2k w - - 0 1").unwrap();
        let new_pos = pos.make_uci_move("a7a8q").unwrap();

        // Should have a queen on a8, not a pawn
        assert!(new_pos
            .piece_bb(Color::White, PieceType::Queen)
            .contains(Square::A8));
        assert!(!new_pos
            .piece_bb(Color::White, PieceType::Pawn)
            .contains(Square::A8));
    }

    #[test]
    fn test_hash_consistency() {
        setup();
        let pos = Position::new();
        let new_pos = pos.make_uci_move("e2e4").unwrap();

        // Hash should match recomputed hash
        assert_eq!(new_pos.hash, new_pos.compute_hash());
    }

    #[test]
    fn test_hash_changes_on_move() {
        setup();
        let pos = Position::new();
        let new_pos = pos.make_uci_move("e2e4").unwrap();

        assert_ne!(pos.hash, new_pos.hash);
    }

    #[test]
    fn test_null_move() {
        setup();
        let pos = Position::new();
        let null_pos = pos.make_null_move();

        assert_eq!(null_pos.side_to_move, Color::Black);
        assert!(null_pos.en_passant.is_none());
    }
}

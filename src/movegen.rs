/// Move generation
use crate::bitboard::{aligned, between, king_attacks, knight_attacks, pawn_attacks, Bitboard};
use crate::magic::{bishop_attacks, rook_attacks};
use crate::moves::{Move, MoveList};
use crate::position::Position;
use crate::types::{CastlingRights, Color, PieceType, Square};

impl Position {
    /// Generate all legal moves
    pub fn generate_legal_moves(&self, list: &mut MoveList) {
        let initial_len = list.len();

        if self.checkers.is_empty() {
            self.generate_moves::<false>(list);
        } else if self.checkers.exactly_one() {
            // Single check - can block or capture checker
            self.generate_moves::<true>(list);
        } else {
            // Double check - only king moves are legal
            self.generate_king_moves(list);
        }

        // Runtime validation: all generated moves should be for the side to move
        for i in initial_len..list.len() {
            let mv = list.get(i);
            if let Some(piece) = self.piece_at(mv.from_sq()) {
                if piece.color() != self.side_to_move {
                    eprintln!(
                        "BUG: generate_legal_moves produced {} for {:?} but side is {:?}",
                        mv.to_uci(),
                        piece.color(),
                        self.side_to_move
                    );
                    eprintln!("Position: {}", self.to_fen());
                }
            }
        }
    }

    /// Generate all pseudo-legal moves (for perft without legality check)
    pub fn generate_pseudo_legal_moves(&self, list: &mut MoveList) {
        self.generate_moves::<false>(list);
    }

    /// Generate capture moves only (for quiescence search)
    pub fn generate_captures(&self, list: &mut MoveList) {
        let us = self.side_to_move;
        let them = us.flip();
        let their_pieces = self.occupied[them as usize];

        // Generate pawn captures
        self.generate_pawn_captures(list, their_pieces);

        // Generate piece captures
        let our_pieces = self.occupied[us as usize];

        // Knights
        for from in self.piece_bb(us, PieceType::Knight) {
            let attacks = knight_attacks(from) & their_pieces;
            for to in attacks {
                list.push(Move::capture(from, to));
            }
        }

        // Bishops
        for from in self.piece_bb(us, PieceType::Bishop) {
            let attacks = bishop_attacks(from, self.all_occupied) & their_pieces;
            for to in attacks {
                list.push(Move::capture(from, to));
            }
        }

        // Rooks
        for from in self.piece_bb(us, PieceType::Rook) {
            let attacks = rook_attacks(from, self.all_occupied) & their_pieces;
            for to in attacks {
                list.push(Move::capture(from, to));
            }
        }

        // Queens
        for from in self.piece_bb(us, PieceType::Queen) {
            let attacks = (bishop_attacks(from, self.all_occupied)
                | rook_attacks(from, self.all_occupied))
                & their_pieces;
            for to in attacks {
                list.push(Move::capture(from, to));
            }
        }

        // King captures
        let king_sq = self.king_sq[us as usize];
        let attacks = king_attacks(king_sq) & their_pieces;
        for to in attacks {
            list.push(Move::capture(king_sq, to));
        }
    }

    /// Generate moves with optional evasion mode
    fn generate_moves<const EVASIONS: bool>(&self, list: &mut MoveList) {
        let us = self.side_to_move;
        let them = us.flip();
        let our_pieces = self.occupied[us as usize];
        let their_pieces = self.occupied[them as usize];
        let empty = !self.all_occupied;

        // Target squares for non-king pieces
        let target = if EVASIONS {
            // In check: can only capture the checker or block
            let checker_sq = self.checkers.lsb();
            let king_sq = self.king_sq[us as usize];
            between(king_sq, checker_sq) | self.checkers
        } else {
            !our_pieces
        };

        let pinned = self.pinned_pieces(us);
        let king_sq = self.king_sq[us as usize];

        // Generate pawn moves
        self.generate_pawn_moves(list, target, pinned, king_sq);

        // Generate knight moves (pinned knights can't move)
        for from in self.piece_bb(us, PieceType::Knight) & !pinned {
            let attacks = knight_attacks(from) & target;
            for to in attacks & their_pieces {
                list.push(Move::capture(from, to));
            }
            for to in attacks & empty {
                list.push(Move::quiet(from, to));
            }
        }

        // Generate bishop moves
        for from in self.piece_bb(us, PieceType::Bishop) {
            let mut attacks = bishop_attacks(from, self.all_occupied) & target;

            // Pinned bishops can only move along pin ray
            if pinned.contains(from) {
                attacks &= crate::bitboard::line(from, king_sq);
            }

            for to in attacks & their_pieces {
                list.push(Move::capture(from, to));
            }
            for to in attacks & empty {
                list.push(Move::quiet(from, to));
            }
        }

        // Generate rook moves
        for from in self.piece_bb(us, PieceType::Rook) {
            let mut attacks = rook_attacks(from, self.all_occupied) & target;

            // Pinned rooks can only move along pin ray
            if pinned.contains(from) {
                attacks &= crate::bitboard::line(from, king_sq);
            }

            for to in attacks & their_pieces {
                list.push(Move::capture(from, to));
            }
            for to in attacks & empty {
                list.push(Move::quiet(from, to));
            }
        }

        // Generate queen moves
        for from in self.piece_bb(us, PieceType::Queen) {
            let mut attacks =
                (bishop_attacks(from, self.all_occupied) | rook_attacks(from, self.all_occupied))
                    & target;

            // Pinned queens can only move along pin ray
            if pinned.contains(from) {
                attacks &= crate::bitboard::line(from, king_sq);
            }

            for to in attacks & their_pieces {
                list.push(Move::capture(from, to));
            }
            for to in attacks & empty {
                list.push(Move::quiet(from, to));
            }
        }

        // Generate king moves
        self.generate_king_moves(list);

        // Generate castling (only when not in check)
        if !EVASIONS {
            self.generate_castling(list);
        }
    }

    /// Generate pawn moves
    fn generate_pawn_moves(
        &self,
        list: &mut MoveList,
        target: Bitboard,
        pinned: Bitboard,
        king_sq: Square,
    ) {
        let us = self.side_to_move;
        let them = us.flip();
        let pawns = self.piece_bb(us, PieceType::Pawn);
        let their_pieces = self.occupied[them as usize];
        let empty = !self.all_occupied;

        let (push_dir, start_rank, promo_rank): (i8, Bitboard, Bitboard) = match us {
            Color::White => (8, Bitboard::RANK_2, Bitboard::RANK_7),
            Color::Black => (-8, Bitboard::RANK_7, Bitboard::RANK_2),
        };

        let promo_pawns = pawns & promo_rank;
        let non_promo_pawns = pawns & !promo_rank;

        // Single pushes (non-promoting)
        let single_push = non_promo_pawns.pawn_push(us) & empty;
        for to in single_push & target {
            let from = Square((to.0 as i8 - push_dir) as u8);
            // Check if pinned
            if !pinned.contains(from) || aligned(from, to, king_sq) {
                list.push(Move::quiet(from, to));
            }
        }

        // Double pushes
        let double_push = (single_push & match us {
            Color::White => Bitboard::RANK_3,
            Color::Black => Bitboard::RANK_6,
        })
        .pawn_push(us)
            & empty
            & target;
        for to in double_push {
            let from = Square((to.0 as i8 - 2 * push_dir) as u8);
            if !pinned.contains(from) || aligned(from, to, king_sq) {
                list.push(Move::double_push(from, to));
            }
        }

        // Promotion pushes
        let promo_push = promo_pawns.pawn_push(us) & empty & target;
        for to in promo_push {
            let from = Square((to.0 as i8 - push_dir) as u8);
            if !pinned.contains(from) || aligned(from, to, king_sq) {
                self.add_promotions(list, from, to, false);
            }
        }

        // Captures (non-promoting)
        let capture_target = their_pieces & target;

        // Left captures
        let left_captures = match us {
            Color::White => non_promo_pawns.north_west(),
            Color::Black => non_promo_pawns.south_west(),
        } & capture_target;
        for to in left_captures {
            let from = Square((to.0 as i8 - push_dir + 1) as u8);
            if !pinned.contains(from) || aligned(from, to, king_sq) {
                list.push(Move::capture(from, to));
            }
        }

        // Right captures
        let right_captures = match us {
            Color::White => non_promo_pawns.north_east(),
            Color::Black => non_promo_pawns.south_east(),
        } & capture_target;
        for to in right_captures {
            let from = Square((to.0 as i8 - push_dir - 1) as u8);
            if !pinned.contains(from) || aligned(from, to, king_sq) {
                list.push(Move::capture(from, to));
            }
        }

        // Promotion captures (left)
        let promo_left_captures = match us {
            Color::White => promo_pawns.north_west(),
            Color::Black => promo_pawns.south_west(),
        } & their_pieces
            & target;
        for to in promo_left_captures {
            let from = Square((to.0 as i8 - push_dir + 1) as u8);
            if !pinned.contains(from) || aligned(from, to, king_sq) {
                self.add_promotions(list, from, to, true);
            }
        }

        // Promotion captures (right)
        let promo_right_captures = match us {
            Color::White => promo_pawns.north_east(),
            Color::Black => promo_pawns.south_east(),
        } & their_pieces
            & target;
        for to in promo_right_captures {
            let from = Square((to.0 as i8 - push_dir - 1) as u8);
            if !pinned.contains(from) || aligned(from, to, king_sq) {
                self.add_promotions(list, from, to, true);
            }
        }

        // En passant
        if let Some(ep_sq) = self.en_passant {
            let attackers = pawn_attacks(them, ep_sq) & pawns;
            for from in attackers {
                // En passant is tricky - need to check if it reveals check
                if self.is_ep_legal(from, ep_sq) {
                    list.push(Move::en_passant(from, ep_sq));
                }
            }
        }
    }

    /// Generate pawn captures only (for quiescence)
    fn generate_pawn_captures(&self, list: &mut MoveList, their_pieces: Bitboard) {
        let us = self.side_to_move;
        let them = us.flip();
        let pawns = self.piece_bb(us, PieceType::Pawn);

        let (push_dir, promo_rank): (i8, Bitboard) = match us {
            Color::White => (8, Bitboard::RANK_7),
            Color::Black => (-8, Bitboard::RANK_2),
        };

        let promo_pawns = pawns & promo_rank;
        let non_promo_pawns = pawns & !promo_rank;

        // Non-promotion captures
        let left_captures = match us {
            Color::White => non_promo_pawns.north_west(),
            Color::Black => non_promo_pawns.south_west(),
        } & their_pieces;
        for to in left_captures {
            let from = Square((to.0 as i8 - push_dir + 1) as u8);
            list.push(Move::capture(from, to));
        }

        let right_captures = match us {
            Color::White => non_promo_pawns.north_east(),
            Color::Black => non_promo_pawns.south_east(),
        } & their_pieces;
        for to in right_captures {
            let from = Square((to.0 as i8 - push_dir - 1) as u8);
            list.push(Move::capture(from, to));
        }

        // Promotion captures
        let promo_left_captures = match us {
            Color::White => promo_pawns.north_west(),
            Color::Black => promo_pawns.south_west(),
        } & their_pieces;
        for to in promo_left_captures {
            let from = Square((to.0 as i8 - push_dir + 1) as u8);
            self.add_promotions(list, from, to, true);
        }

        let promo_right_captures = match us {
            Color::White => promo_pawns.north_east(),
            Color::Black => promo_pawns.south_east(),
        } & their_pieces;
        for to in promo_right_captures {
            let from = Square((to.0 as i8 - push_dir - 1) as u8);
            self.add_promotions(list, from, to, true);
        }

        // Promotion pushes (considered tactical)
        let empty = !self.all_occupied;
        let promo_push = promo_pawns.pawn_push(us) & empty;
        for to in promo_push {
            let from = Square((to.0 as i8 - push_dir) as u8);
            self.add_promotions(list, from, to, false);
        }

        // En passant captures
        if let Some(ep_sq) = self.en_passant {
            let attackers = pawn_attacks(them, ep_sq) & pawns;
            for from in attackers {
                if self.is_ep_legal(from, ep_sq) {
                    list.push(Move::en_passant(from, ep_sq));
                }
            }
        }
    }

    /// Add all four promotion moves
    fn add_promotions(&self, list: &mut MoveList, from: Square, to: Square, is_capture: bool) {
        list.push(Move::promotion(from, to, PieceType::Queen, is_capture));
        list.push(Move::promotion(from, to, PieceType::Rook, is_capture));
        list.push(Move::promotion(from, to, PieceType::Bishop, is_capture));
        list.push(Move::promotion(from, to, PieceType::Knight, is_capture));
    }

    /// Generate king moves (excluding castling)
    fn generate_king_moves(&self, list: &mut MoveList) {
        let us = self.side_to_move;
        let them = us.flip();
        let king_sq = self.king_sq[us as usize];
        let our_pieces = self.occupied[us as usize];
        let their_pieces = self.occupied[them as usize];

        let attacks = king_attacks(king_sq) & !our_pieces;

        // For each potential king move, check if destination is attacked
        for to in attacks {
            // Temporarily remove king to check if square is attacked
            let occupied_without_king = self.all_occupied.clear(king_sq);
            if !self.is_square_attacked(to, them, occupied_without_king) {
                if their_pieces.contains(to) {
                    list.push(Move::capture(king_sq, to));
                } else {
                    list.push(Move::quiet(king_sq, to));
                }
            }
        }
    }

    /// Generate castling moves
    fn generate_castling(&self, list: &mut MoveList) {
        let us = self.side_to_move;
        let them = us.flip();

        let (king_sq, ks_target, qs_target, ks_path, qs_path, ks_check_path, qs_check_path) =
            match us {
                Color::White => (
                    Square::E1,
                    Square::G1,
                    Square::C1,
                    Bitboard::new(0x60),               // f1, g1
                    Bitboard::new(0x0E),               // b1, c1, d1
                    Bitboard::new(0x60),               // f1, g1
                    Bitboard::new(0x0C),               // c1, d1
                ),
                Color::Black => (
                    Square::E8,
                    Square::G8,
                    Square::C8,
                    Bitboard::new(0x6000000000000000), // f8, g8
                    Bitboard::new(0x0E00000000000000), // b8, c8, d8
                    Bitboard::new(0x6000000000000000), // f8, g8
                    Bitboard::new(0x0C00000000000000), // c8, d8
                ),
            };

        // Kingside castling
        if self.castling.contains(CastlingRights::kingside(us)) {
            // Path must be clear
            if (self.all_occupied & ks_path).is_empty() {
                // King and path must not be attacked
                if !self.is_attacked_by(king_sq, them)
                    && !self.any_attacked(ks_check_path, them)
                {
                    list.push(Move::king_castle(king_sq, ks_target));
                }
            }
        }

        // Queenside castling
        if self.castling.contains(CastlingRights::queenside(us)) {
            // Path must be clear
            if (self.all_occupied & qs_path).is_empty() {
                // King and path must not be attacked
                if !self.is_attacked_by(king_sq, them)
                    && !self.any_attacked(qs_check_path, them)
                {
                    list.push(Move::queen_castle(king_sq, qs_target));
                }
            }
        }
    }

    /// Check if any square in a bitboard is attacked by a color
    fn any_attacked(&self, squares: Bitboard, by_color: Color) -> bool {
        for sq in squares {
            if self.is_attacked_by(sq, by_color) {
                return true;
            }
        }
        false
    }

    /// Check if a square is attacked (with custom occupancy)
    fn is_square_attacked(&self, sq: Square, by_color: Color, occupied: Bitboard) -> bool {
        self.attackers_to_by(sq, by_color, occupied).is_not_empty()
    }

    /// Check if en passant is legal (doesn't reveal check)
    fn is_ep_legal(&self, from: Square, ep_sq: Square) -> bool {
        let us = self.side_to_move;
        let them = us.flip();
        let king_sq = self.king_sq[us as usize];

        // The captured pawn square
        let captured_sq = Square((ep_sq.0 as i8 + if us == Color::White { -8 } else { 8 }) as u8);

        // Remove both pawns and add capturing pawn at destination
        let occupied = self
            .all_occupied
            .clear(from)
            .clear(captured_sq)
            .set(ep_sq);

        // Check if king is attacked after the move
        let rook_attacks = rook_attacks(king_sq, occupied);
        let bishop_att = bishop_attacks(king_sq, occupied);

        let enemy_rooks = self.orthogonal_sliders(them);
        let enemy_bishops = self.diagonal_sliders(them);

        (rook_attacks & enemy_rooks).is_empty() && (bishop_att & enemy_bishops).is_empty()
    }

    /// Check if a move is legal
    pub fn is_legal(&self, mv: Move) -> bool {
        let us = self.side_to_move;
        let them = us.flip();
        let from = mv.from_sq();
        let to = mv.to_sq();
        let king_sq = self.king_sq[us as usize];

        // King moves require destination to not be attacked
        if from == king_sq {
            if mv.is_castle() {
                // Castling legality is checked during generation
                return true;
            }
            let occupied_without_king = self.all_occupied.clear(from);
            return !self.is_square_attacked(to, them, occupied_without_king);
        }

        // En passant requires special check
        if mv.is_en_passant() {
            return self.is_ep_legal(from, to);
        }

        // Non-king moves: check if piece is pinned
        let pinned = self.pinned_pieces(us);
        if pinned.contains(from) {
            // Pinned piece can only move along pin ray
            return aligned(from, to, king_sq);
        }

        // If in check, verify move blocks or captures
        if self.checkers.is_not_empty() {
            let checker_sq = self.checkers.lsb();
            let block_mask = between(king_sq, checker_sq) | self.checkers;
            return block_mask.contains(to);
        }

        true
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
    fn test_startpos_moves() {
        setup();
        let pos = Position::new();
        let mut list = MoveList::new();
        pos.generate_legal_moves(&mut list);
        assert_eq!(list.len(), 20); // 16 pawn moves + 4 knight moves
    }

    #[test]
    fn test_kiwipete_moves() {
        setup();
        let pos = Position::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        )
        .unwrap();
        let mut list = MoveList::new();
        pos.generate_legal_moves(&mut list);
        assert_eq!(list.len(), 48);
    }

    #[test]
    fn test_castling_generation() {
        setup();
        let pos =
            Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        let mut list = MoveList::new();
        pos.generate_legal_moves(&mut list);

        let has_kingside = list.iter().any(|m| m.is_kingside_castle());
        let has_queenside = list.iter().any(|m| m.is_queenside_castle());
        assert!(has_kingside, "Should have kingside castling");
        assert!(has_queenside, "Should have queenside castling");
    }

    #[test]
    fn test_no_castling_through_check() {
        setup();
        // Rook on e7 attacks e1
        let pos =
            Position::from_fen("4k3/4r3/8/8/8/8/8/R3K2R w KQ - 0 1").unwrap();
        let mut list = MoveList::new();
        pos.generate_legal_moves(&mut list);

        let has_kingside = list.iter().any(|m| m.is_kingside_castle());
        let has_queenside = list.iter().any(|m| m.is_queenside_castle());
        assert!(!has_kingside, "Should not castle through check (kingside)");
        assert!(!has_queenside, "Should not castle through check (queenside)");
    }

    #[test]
    fn test_en_passant() {
        setup();
        let pos =
            Position::from_fen("rnbqkbnr/pppp1ppp/8/4pP2/8/8/PPPPP1PP/RNBQKBNR w KQkq e6 0 1")
                .unwrap();
        let mut list = MoveList::new();
        pos.generate_legal_moves(&mut list);

        let has_ep = list.iter().any(|m| m.is_en_passant());
        assert!(has_ep, "Should have en passant capture");
    }

    #[test]
    fn test_promotion() {
        setup();
        let pos = Position::from_fen("8/P7/8/8/8/8/8/4K2k w - - 0 1").unwrap();
        let mut list = MoveList::new();
        pos.generate_legal_moves(&mut list);

        let promos: Vec<Move> = list.iter().filter(|m| m.is_promotion()).collect();
        assert_eq!(promos.len(), 4, "Should have 4 promotion options");
    }

    #[test]
    fn test_double_check() {
        setup();
        // Double check position - only king moves are legal
        let pos =
            Position::from_fen("r1bqk2r/pppp1Npp/2n2n2/2b1p3/2B1P3/8/PPPP1PPP/RNBQK2R b KQkq - 0 1")
                .unwrap();
        let mut list = MoveList::new();
        pos.generate_legal_moves(&mut list);

        // All moves should be king moves when in double check
        for mv in list.iter() {
            assert_eq!(
                mv.from_sq(),
                pos.king_sq[Color::Black as usize],
                "In double check, only king can move"
            );
        }
    }
}

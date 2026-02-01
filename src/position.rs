/// Board representation and FEN parsing
use crate::bitboard::{king_attacks, knight_attacks, pawn_attacks, Bitboard, KING_ATTACKS};
use crate::magic::{bishop_attacks, queen_attacks, rook_attacks};
use crate::types::{CastlingRights, Color, Piece, PieceType, Square};
use crate::zobrist::ZOBRIST;

/// Represents a chess position
#[derive(Clone)]
pub struct Position {
    /// Piece bitboards: [color][piece_type]
    pub pieces: [[Bitboard; 6]; 2],

    /// Occupancy bitboards per color
    pub occupied: [Bitboard; 2],

    /// All occupied squares
    pub all_occupied: Bitboard,

    /// Mailbox representation for quick piece lookup
    pub board: [Option<Piece>; 64],

    /// Side to move
    pub side_to_move: Color,

    /// Castling rights
    pub castling: CastlingRights,

    /// En passant target square (if any)
    pub en_passant: Option<Square>,

    /// Halfmove clock (for 50-move rule)
    pub halfmove_clock: u8,

    /// Fullmove number
    pub fullmove_number: u16,

    /// Zobrist hash key
    pub hash: u64,

    /// King squares (cached for quick access)
    pub king_sq: [Square; 2],

    /// Checkers bitboard (pieces giving check)
    pub checkers: Bitboard,
}

impl Position {
    /// Standard starting position FEN
    pub const STARTPOS: &'static str =
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    /// Create an empty position
    pub fn empty() -> Self {
        Position {
            pieces: [[Bitboard::EMPTY; 6]; 2],
            occupied: [Bitboard::EMPTY; 2],
            all_occupied: Bitboard::EMPTY,
            board: [None; 64],
            side_to_move: Color::White,
            castling: CastlingRights::NONE,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            hash: 0,
            king_sq: [Square::E1, Square::E8],
            checkers: Bitboard::EMPTY,
        }
    }

    /// Create a new position from the starting position
    pub fn new() -> Self {
        Self::from_fen(Self::STARTPOS).expect("Invalid starting position FEN")
    }

    /// Parse a position from FEN string
    pub fn from_fen(fen: &str) -> Result<Self, &'static str> {
        let mut pos = Self::empty();
        let parts: Vec<&str> = fen.split_whitespace().collect();

        if parts.is_empty() {
            return Err("Empty FEN string");
        }

        // Parse piece placement
        let mut sq = 56u8; // Start at a8
        for c in parts[0].chars() {
            match c {
                '/' => {
                    sq = sq.wrapping_sub(16); // Move to next rank down
                }
                '1'..='8' => {
                    sq += (c as u8) - b'0';
                }
                _ => {
                    if let Some(piece) = Piece::from_char(c) {
                        pos.put_piece(Square(sq), piece);
                        sq += 1;
                    } else {
                        return Err("Invalid piece character in FEN");
                    }
                }
            }
        }

        // Parse side to move
        if parts.len() > 1 {
            pos.side_to_move = match parts[1] {
                "w" => Color::White,
                "b" => Color::Black,
                _ => return Err("Invalid side to move"),
            };
        }

        // Parse castling rights
        if parts.len() > 2 {
            pos.castling = CastlingRights::from_fen(parts[2]);
        }

        // Parse en passant square
        if parts.len() > 3 && parts[3] != "-" {
            pos.en_passant = Square::from_algebraic(parts[3]);
        }

        // Parse halfmove clock
        if parts.len() > 4 {
            pos.halfmove_clock = parts[4].parse().unwrap_or(0);
        }

        // Parse fullmove number
        if parts.len() > 5 {
            pos.fullmove_number = parts[5].parse().unwrap_or(1);
        }

        // Compute hash
        pos.hash = pos.compute_hash();

        // Compute checkers
        pos.checkers = pos.compute_checkers();

        Ok(pos)
    }

    /// Convert position to FEN string
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        // Piece placement
        for rank in (0..8).rev() {
            let mut empty_count = 0;

            for file in 0..8 {
                let sq = Square::from_coords(file, rank);
                if let Some(piece) = self.board[sq.0 as usize] {
                    if empty_count > 0 {
                        fen.push((b'0' + empty_count) as char);
                        empty_count = 0;
                    }
                    fen.push(piece.to_char());
                } else {
                    empty_count += 1;
                }
            }

            if empty_count > 0 {
                fen.push((b'0' + empty_count) as char);
            }

            if rank > 0 {
                fen.push('/');
            }
        }

        // Side to move
        fen.push(' ');
        fen.push(match self.side_to_move {
            Color::White => 'w',
            Color::Black => 'b',
        });

        // Castling rights
        fen.push(' ');
        fen.push_str(&self.castling.to_fen());

        // En passant
        fen.push(' ');
        match self.en_passant {
            Some(sq) => fen.push_str(&sq.to_algebraic()),
            None => fen.push('-'),
        }

        // Halfmove clock and fullmove number
        fen.push(' ');
        fen.push_str(&self.halfmove_clock.to_string());
        fen.push(' ');
        fen.push_str(&self.fullmove_number.to_string());

        fen
    }

    /// Put a piece on a square
    pub fn put_piece(&mut self, sq: Square, piece: Piece) {
        let color = piece.color();
        let piece_type = piece.piece_type();

        self.pieces[color as usize][piece_type as usize] =
            self.pieces[color as usize][piece_type as usize].set(sq);
        self.occupied[color as usize] = self.occupied[color as usize].set(sq);
        self.all_occupied = self.all_occupied.set(sq);
        self.board[sq.0 as usize] = Some(piece);

        if piece_type == PieceType::King {
            self.king_sq[color as usize] = sq;
        }
    }

    /// Remove a piece from a square
    pub fn remove_piece(&mut self, sq: Square) -> Option<Piece> {
        if let Some(piece) = self.board[sq.0 as usize] {
            let color = piece.color();
            let piece_type = piece.piece_type();

            self.pieces[color as usize][piece_type as usize] =
                self.pieces[color as usize][piece_type as usize].clear(sq);
            self.occupied[color as usize] = self.occupied[color as usize].clear(sq);
            self.all_occupied = self.all_occupied.clear(sq);
            self.board[sq.0 as usize] = None;

            Some(piece)
        } else {
            None
        }
    }

    /// Get piece bitboard for a color and piece type
    #[inline(always)]
    pub fn piece_bb(&self, color: Color, piece_type: PieceType) -> Bitboard {
        self.pieces[color as usize][piece_type as usize]
    }

    /// Get all pieces of a color
    #[inline(always)]
    pub fn pieces_of(&self, color: Color) -> Bitboard {
        self.occupied[color as usize]
    }

    /// Get the piece at a square
    #[inline(always)]
    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        self.board[sq.0 as usize]
    }

    /// Check if the side to move is in check
    #[inline(always)]
    pub fn is_in_check(&self) -> bool {
        self.checkers.is_not_empty()
    }

    /// Get diagonal sliders (bishops and queens)
    #[inline(always)]
    pub fn diagonal_sliders(&self, color: Color) -> Bitboard {
        self.piece_bb(color, PieceType::Bishop) | self.piece_bb(color, PieceType::Queen)
    }

    /// Get orthogonal sliders (rooks and queens)
    #[inline(always)]
    pub fn orthogonal_sliders(&self, color: Color) -> Bitboard {
        self.piece_bb(color, PieceType::Rook) | self.piece_bb(color, PieceType::Queen)
    }

    /// Get all attackers to a square
    pub fn attackers_to(&self, sq: Square, occupied: Bitboard) -> Bitboard {
        let knights =
            self.piece_bb(Color::White, PieceType::Knight) | self.piece_bb(Color::Black, PieceType::Knight);
        let kings =
            self.piece_bb(Color::White, PieceType::King) | self.piece_bb(Color::Black, PieceType::King);
        let diag_sliders = self.diagonal_sliders(Color::White) | self.diagonal_sliders(Color::Black);
        let orth_sliders =
            self.orthogonal_sliders(Color::White) | self.orthogonal_sliders(Color::Black);

        let white_pawns = self.piece_bb(Color::White, PieceType::Pawn);
        let black_pawns = self.piece_bb(Color::Black, PieceType::Pawn);

        (pawn_attacks(Color::Black, sq) & white_pawns)
            | (pawn_attacks(Color::White, sq) & black_pawns)
            | (knight_attacks(sq) & knights)
            | (king_attacks(sq) & kings)
            | (bishop_attacks(sq, occupied) & diag_sliders)
            | (rook_attacks(sq, occupied) & orth_sliders)
    }

    /// Get attackers of a specific color to a square
    pub fn attackers_to_by(&self, sq: Square, color: Color, occupied: Bitboard) -> Bitboard {
        let pawns = self.piece_bb(color, PieceType::Pawn);
        let knights = self.piece_bb(color, PieceType::Knight);
        let bishops = self.piece_bb(color, PieceType::Bishop);
        let rooks = self.piece_bb(color, PieceType::Rook);
        let queens = self.piece_bb(color, PieceType::Queen);
        let kings = self.piece_bb(color, PieceType::King);

        (pawn_attacks(color.flip(), sq) & pawns)
            | (knight_attacks(sq) & knights)
            | (bishop_attacks(sq, occupied) & (bishops | queens))
            | (rook_attacks(sq, occupied) & (rooks | queens))
            | (king_attacks(sq) & kings)
    }

    /// Check if a square is attacked by a color
    #[inline(always)]
    pub fn is_attacked_by(&self, sq: Square, color: Color) -> bool {
        self.attackers_to_by(sq, color, self.all_occupied)
            .is_not_empty()
    }

    /// Compute checkers bitboard
    pub fn compute_checkers(&self) -> Bitboard {
        let us = self.side_to_move;
        let king_sq = self.king_sq[us as usize];
        self.attackers_to_by(king_sq, us.flip(), self.all_occupied)
    }

    /// Compute the full Zobrist hash from scratch
    pub fn compute_hash(&self) -> u64 {
        let mut hash = 0u64;

        // Piece keys
        for color in [Color::White, Color::Black] {
            for piece_type in [
                PieceType::Pawn,
                PieceType::Knight,
                PieceType::Bishop,
                PieceType::Rook,
                PieceType::Queen,
                PieceType::King,
            ] {
                let mut bb = self.piece_bb(color, piece_type);
                while bb.is_not_empty() {
                    let sq = bb.pop_lsb();
                    hash ^= ZOBRIST.piece_key(color, piece_type, sq);
                }
            }
        }

        // Castling key
        hash ^= ZOBRIST.castling_key(self.castling);

        // En passant key
        if let Some(ep_sq) = self.en_passant {
            hash ^= ZOBRIST.en_passant_key(ep_sq.file());
        }

        // Side to move
        if self.side_to_move == Color::Black {
            hash ^= ZOBRIST.side_key();
        }

        hash
    }

    /// Get pieces that are pinned to the king
    pub fn pinned_pieces(&self, color: Color) -> Bitboard {
        let king_sq = self.king_sq[color as usize];
        let them = color.flip();
        let our_pieces = self.occupied[color as usize];

        let mut pinned = Bitboard::EMPTY;

        // Check diagonal pins (bishops and queens)
        let diag_attackers = bishop_attacks(king_sq, self.occupied[them as usize])
            & self.diagonal_sliders(them);
        for attacker in diag_attackers {
            let between = crate::bitboard::between(king_sq, attacker) & self.all_occupied;
            if between.exactly_one() {
                pinned |= between & our_pieces;
            }
        }

        // Check orthogonal pins (rooks and queens)
        let orth_attackers = rook_attacks(king_sq, self.occupied[them as usize])
            & self.orthogonal_sliders(them);
        for attacker in orth_attackers {
            let between = crate::bitboard::between(king_sq, attacker) & self.all_occupied;
            if between.exactly_one() {
                pinned |= between & our_pieces;
            }
        }

        pinned
    }

    /// Print the board (for debugging)
    pub fn print(&self) {
        println!();
        for rank in (0..8).rev() {
            print!("  {} ", rank + 1);
            for file in 0..8 {
                let sq = Square::from_coords(file, rank);
                let c = match self.board[sq.0 as usize] {
                    Some(piece) => piece.to_char(),
                    None => '.',
                };
                print!("{} ", c);
            }
            println!();
        }
        println!("    a b c d e f g h");
        println!();
        println!("  FEN: {}", self.to_fen());
        println!("  Hash: 0x{:016X}", self.hash);
        println!(
            "  Checkers: {}",
            if self.checkers.is_empty() {
                "none".to_string()
            } else {
                format!("{:?}", self.checkers)
            }
        );
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for rank in (0..8).rev() {
            write!(f, "  {} ", rank + 1)?;
            for file in 0..8 {
                let sq = Square::from_coords(file, rank);
                let c = match self.board[sq.0 as usize] {
                    Some(piece) => piece.to_char(),
                    None => '.',
                };
                write!(f, "{} ", c)?;
            }
            writeln!(f)?;
        }
        writeln!(f, "    a b c d e f g h")?;
        writeln!(f, "  FEN: {}", self.to_fen())
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
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
    fn test_startpos() {
        setup();
        let pos = Position::new();
        assert_eq!(pos.side_to_move, Color::White);
        assert_eq!(pos.castling, CastlingRights::ALL);
        assert!(pos.en_passant.is_none());
        assert_eq!(pos.halfmove_clock, 0);
        assert_eq!(pos.fullmove_number, 1);
    }

    #[test]
    fn test_fen_roundtrip() {
        setup();
        let fens = [
            Position::STARTPOS,
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        ];

        for fen in fens {
            let pos = Position::from_fen(fen).unwrap();
            let regenerated = pos.to_fen();
            assert_eq!(fen, regenerated, "FEN roundtrip failed for: {}", fen);
        }
    }

    #[test]
    fn test_piece_bitboards() {
        setup();
        let pos = Position::new();

        // Check white pawns on rank 2
        let white_pawns = pos.piece_bb(Color::White, PieceType::Pawn);
        assert_eq!(white_pawns.pop_count(), 8);
        assert_eq!(white_pawns, Bitboard::RANK_2);

        // Check black pawns on rank 7
        let black_pawns = pos.piece_bb(Color::Black, PieceType::Pawn);
        assert_eq!(black_pawns.pop_count(), 8);
        assert_eq!(black_pawns, Bitboard::RANK_7);

        // Check kings
        assert!(pos
            .piece_bb(Color::White, PieceType::King)
            .contains(Square::E1));
        assert!(pos
            .piece_bb(Color::Black, PieceType::King)
            .contains(Square::E8));
    }

    #[test]
    fn test_is_attacked() {
        setup();
        let pos = Position::new();

        // e2 pawn attacks d3 and f3
        assert!(pos.is_attacked_by(Square::from_algebraic("d3").unwrap(), Color::White));
        assert!(pos.is_attacked_by(Square::from_algebraic("f3").unwrap(), Color::White));

        // Knights attack various squares
        assert!(pos.is_attacked_by(Square::from_algebraic("c3").unwrap(), Color::White));
        assert!(pos.is_attacked_by(Square::from_algebraic("f3").unwrap(), Color::White));
    }

    #[test]
    fn test_hash_stability() {
        setup();
        let pos1 = Position::new();
        let pos2 = Position::new();
        assert_eq!(pos1.hash, pos2.hash);

        // Different positions should have different hashes
        let pos3 = Position::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1")
            .unwrap();
        assert_ne!(pos1.hash, pos3.hash);
    }
}

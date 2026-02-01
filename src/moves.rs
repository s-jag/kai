/// Move encoding and representation
use crate::types::{PieceType, Square};

/// Move encoding (16 bits):
/// bits 0-5:   from square (0-63)
/// bits 6-11:  to square (0-63)
/// bits 12-15: flags
#[derive(Clone, Copy, PartialEq, Eq, Default, Hash)]
#[repr(transparent)]
pub struct Move(pub u16);

impl Move {
    /// Null move constant
    pub const NULL: Move = Move(0);

    // Move flags
    pub const FLAG_QUIET: u16 = 0b0000;
    pub const FLAG_DOUBLE_PUSH: u16 = 0b0001;
    pub const FLAG_KING_CASTLE: u16 = 0b0010;
    pub const FLAG_QUEEN_CASTLE: u16 = 0b0011;
    pub const FLAG_CAPTURE: u16 = 0b0100;
    pub const FLAG_EP_CAPTURE: u16 = 0b0101;
    // 0b0110 and 0b0111 unused
    pub const FLAG_PROMO_KNIGHT: u16 = 0b1000;
    pub const FLAG_PROMO_BISHOP: u16 = 0b1001;
    pub const FLAG_PROMO_ROOK: u16 = 0b1010;
    pub const FLAG_PROMO_QUEEN: u16 = 0b1011;
    pub const FLAG_PROMO_CAPTURE_KNIGHT: u16 = 0b1100;
    pub const FLAG_PROMO_CAPTURE_BISHOP: u16 = 0b1101;
    pub const FLAG_PROMO_CAPTURE_ROOK: u16 = 0b1110;
    pub const FLAG_PROMO_CAPTURE_QUEEN: u16 = 0b1111;

    /// Create a new move
    #[inline(always)]
    pub const fn new(from: Square, to: Square, flags: u16) -> Self {
        Move((from.0 as u16) | ((to.0 as u16) << 6) | (flags << 12))
    }

    /// Create a quiet move
    #[inline(always)]
    pub const fn quiet(from: Square, to: Square) -> Self {
        Self::new(from, to, Self::FLAG_QUIET)
    }

    /// Create a capture move
    #[inline(always)]
    pub const fn capture(from: Square, to: Square) -> Self {
        Self::new(from, to, Self::FLAG_CAPTURE)
    }

    /// Create a double pawn push
    #[inline(always)]
    pub const fn double_push(from: Square, to: Square) -> Self {
        Self::new(from, to, Self::FLAG_DOUBLE_PUSH)
    }

    /// Create an en passant capture
    #[inline(always)]
    pub const fn en_passant(from: Square, to: Square) -> Self {
        Self::new(from, to, Self::FLAG_EP_CAPTURE)
    }

    /// Create a kingside castle
    #[inline(always)]
    pub const fn king_castle(from: Square, to: Square) -> Self {
        Self::new(from, to, Self::FLAG_KING_CASTLE)
    }

    /// Create a queenside castle
    #[inline(always)]
    pub const fn queen_castle(from: Square, to: Square) -> Self {
        Self::new(from, to, Self::FLAG_QUEEN_CASTLE)
    }

    /// Create a promotion move
    #[inline(always)]
    pub const fn promotion(from: Square, to: Square, piece: PieceType, is_capture: bool) -> Self {
        let base = match piece {
            PieceType::Knight => Self::FLAG_PROMO_KNIGHT,
            PieceType::Bishop => Self::FLAG_PROMO_BISHOP,
            PieceType::Rook => Self::FLAG_PROMO_ROOK,
            _ => Self::FLAG_PROMO_QUEEN,
        };
        let flags = if is_capture { base + 4 } else { base };
        Self::new(from, to, flags)
    }

    /// Get the source square
    #[inline(always)]
    pub const fn from_sq(self) -> Square {
        Square((self.0 & 0x3F) as u8)
    }

    /// Get the destination square
    #[inline(always)]
    pub const fn to_sq(self) -> Square {
        Square(((self.0 >> 6) & 0x3F) as u8)
    }

    /// Get the move flags
    #[inline(always)]
    pub const fn flags(self) -> u16 {
        self.0 >> 12
    }

    /// Check if this is a capture
    #[inline(always)]
    pub const fn is_capture(self) -> bool {
        (self.flags() & 0b0100) != 0
    }

    /// Check if this is a promotion
    #[inline(always)]
    pub const fn is_promotion(self) -> bool {
        (self.flags() & 0b1000) != 0
    }

    /// Check if this is a tactical move (capture or promotion)
    #[inline(always)]
    pub const fn is_tactical(self) -> bool {
        (self.flags() & 0b1100) != 0
    }

    /// Check if this is a quiet move (not capture, not promotion)
    #[inline(always)]
    pub const fn is_quiet(self) -> bool {
        !self.is_tactical()
    }

    /// Check if this is a castling move
    #[inline(always)]
    pub const fn is_castle(self) -> bool {
        let f = self.flags();
        f == Self::FLAG_KING_CASTLE || f == Self::FLAG_QUEEN_CASTLE
    }

    /// Check if this is kingside castling
    #[inline(always)]
    pub const fn is_kingside_castle(self) -> bool {
        self.flags() == Self::FLAG_KING_CASTLE
    }

    /// Check if this is queenside castling
    #[inline(always)]
    pub const fn is_queenside_castle(self) -> bool {
        self.flags() == Self::FLAG_QUEEN_CASTLE
    }

    /// Check if this is a double pawn push
    #[inline(always)]
    pub const fn is_double_push(self) -> bool {
        self.flags() == Self::FLAG_DOUBLE_PUSH
    }

    /// Check if this is an en passant capture
    #[inline(always)]
    pub const fn is_en_passant(self) -> bool {
        self.flags() == Self::FLAG_EP_CAPTURE
    }

    /// Check if this is the null move
    #[inline(always)]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Get the promotion piece type (only valid if is_promotion() is true)
    #[inline(always)]
    pub const fn promotion_piece(self) -> PieceType {
        match self.flags() & 0b0011 {
            0 => PieceType::Knight,
            1 => PieceType::Bishop,
            2 => PieceType::Rook,
            _ => PieceType::Queen,
        }
    }

    /// Convert to UCI string (e.g., "e2e4", "e7e8q")
    pub fn to_uci(self) -> String {
        if self.is_null() {
            return "0000".to_string();
        }

        let from = self.from_sq().to_algebraic();
        let to = self.to_sq().to_algebraic();

        if self.is_promotion() {
            let promo = match self.promotion_piece() {
                PieceType::Knight => 'n',
                PieceType::Bishop => 'b',
                PieceType::Rook => 'r',
                PieceType::Queen => 'q',
                _ => 'q',
            };
            format!("{}{}{}", from, to, promo)
        } else {
            format!("{}{}", from, to)
        }
    }
}

impl std::fmt::Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_uci())
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_uci())
    }
}

/// Maximum number of legal moves in any chess position
pub const MAX_MOVES: usize = 256;

/// Stack-allocated move list
pub struct MoveList {
    moves: [Move; MAX_MOVES],
    scores: [i32; MAX_MOVES],
    len: usize,
}

impl MoveList {
    /// Create an empty move list
    #[inline(always)]
    pub fn new() -> Self {
        MoveList {
            moves: [Move::NULL; MAX_MOVES],
            scores: [0; MAX_MOVES],
            len: 0,
        }
    }

    /// Add a move to the list
    #[inline(always)]
    pub fn push(&mut self, mv: Move) {
        debug_assert!(self.len < MAX_MOVES);
        self.moves[self.len] = mv;
        self.len += 1;
    }

    /// Add a move with a score
    #[inline(always)]
    pub fn push_scored(&mut self, mv: Move, score: i32) {
        debug_assert!(self.len < MAX_MOVES);
        self.moves[self.len] = mv;
        self.scores[self.len] = score;
        self.len += 1;
    }

    /// Get the number of moves
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get a move by index
    #[inline(always)]
    pub fn get(&self, index: usize) -> Move {
        debug_assert!(index < self.len);
        self.moves[index]
    }

    /// Get a mutable reference to a move's score
    #[inline(always)]
    pub fn score_mut(&mut self, index: usize) -> &mut i32 {
        debug_assert!(index < self.len);
        &mut self.scores[index]
    }

    /// Get the score for a move
    #[inline(always)]
    pub fn score(&self, index: usize) -> i32 {
        debug_assert!(index < self.len);
        self.scores[index]
    }

    /// Set the score for a move
    #[inline(always)]
    pub fn set_score(&mut self, index: usize, score: i32) {
        debug_assert!(index < self.len);
        self.scores[index] = score;
    }

    /// Swap two moves
    #[inline(always)]
    pub fn swap(&mut self, i: usize, j: usize) {
        self.moves.swap(i, j);
        self.scores.swap(i, j);
    }

    /// Clear the move list
    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Iterate over moves
    pub fn iter(&self) -> impl Iterator<Item = Move> + '_ {
        self.moves[..self.len].iter().copied()
    }

    /// Check if the list contains a move
    pub fn contains(&self, mv: Move) -> bool {
        self.moves[..self.len].contains(&mv)
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Index<usize> for MoveList {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.moves[index]
    }
}

impl std::fmt::Debug for MoveList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for i in 0..self.len {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", self.moves[i].to_uci())?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_encoding() {
        let mv = Move::quiet(Square::E2, Square::E4);
        assert_eq!(mv.from_sq(), Square::E2);
        assert_eq!(mv.to_sq(), Square::E4);
        assert!(!mv.is_capture());
        assert!(!mv.is_promotion());
    }

    #[test]
    fn test_move_capture() {
        let mv = Move::capture(Square::E4, Square::D5);
        assert!(mv.is_capture());
        assert!(!mv.is_promotion());
        assert!(mv.is_tactical());
    }

    #[test]
    fn test_move_promotion() {
        let mv = Move::promotion(Square::E7, Square::E8, PieceType::Queen, false);
        assert!(mv.is_promotion());
        assert!(!mv.is_capture());
        assert_eq!(mv.promotion_piece(), PieceType::Queen);
        assert_eq!(mv.to_uci(), "e7e8q");

        let mv_cap = Move::promotion(Square::E7, Square::D8, PieceType::Knight, true);
        assert!(mv_cap.is_promotion());
        assert!(mv_cap.is_capture());
        assert_eq!(mv_cap.promotion_piece(), PieceType::Knight);
        assert_eq!(mv_cap.to_uci(), "e7d8n");
    }

    #[test]
    fn test_move_castle() {
        let ks = Move::king_castle(Square::E1, Square::G1);
        assert!(ks.is_castle());
        assert!(ks.is_kingside_castle());
        assert!(!ks.is_queenside_castle());

        let qs = Move::queen_castle(Square::E1, Square::C1);
        assert!(qs.is_castle());
        assert!(!qs.is_kingside_castle());
        assert!(qs.is_queenside_castle());
    }

    #[test]
    fn test_move_list() {
        let mut list = MoveList::new();
        assert!(list.is_empty());

        list.push(Move::quiet(Square::E2, Square::E4));
        list.push(Move::quiet(Square::D2, Square::D4));
        assert_eq!(list.len(), 2);

        let moves: Vec<Move> = list.iter().collect();
        assert_eq!(moves.len(), 2);
    }

    #[test]
    fn test_uci_format() {
        assert_eq!(Move::quiet(Square::E2, Square::E4).to_uci(), "e2e4");
        assert_eq!(Move::capture(Square::E4, Square::D5).to_uci(), "e4d5");
        assert_eq!(Move::king_castle(Square::E1, Square::G1).to_uci(), "e1g1");
        assert_eq!(Move::NULL.to_uci(), "0000");
    }
}

// Re-export Square constants we need
impl Square {
    pub const E2: Square = Square(12);
    pub const E4: Square = Square(28);
    pub const D5: Square = Square(35);
    pub const E7: Square = Square(52);
    pub const D8: Square = Square(59);
    pub const G1: Square = Square(6);
    pub const C1: Square = Square(2);
}

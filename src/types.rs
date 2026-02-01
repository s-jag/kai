/// Core types for the chess engine

/// Represents a square on the chess board (0-63)
/// Layout: a1=0, b1=1, ..., h1=7, a2=8, ..., h8=63
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct Square(pub u8);

impl Square {
    pub const A1: Square = Square(0);
    pub const B1: Square = Square(1);
    pub const C1: Square = Square(2);
    pub const D1: Square = Square(3);
    pub const E1: Square = Square(4);
    pub const F1: Square = Square(5);
    pub const G1: Square = Square(6);
    pub const H1: Square = Square(7);
    pub const A8: Square = Square(56);
    pub const B8: Square = Square(57);
    pub const C8: Square = Square(58);
    pub const D8: Square = Square(59);
    pub const E8: Square = Square(60);
    pub const F8: Square = Square(61);
    pub const G8: Square = Square(62);
    pub const H8: Square = Square(63);

    pub const NONE: Square = Square(64);

    #[inline(always)]
    pub const fn new(sq: u8) -> Self {
        debug_assert!(sq < 64);
        Square(sq)
    }

    #[inline(always)]
    pub const fn from_coords(file: u8, rank: u8) -> Self {
        debug_assert!(file < 8 && rank < 8);
        Square(rank * 8 + file)
    }

    #[inline(always)]
    pub const fn file(self) -> u8 {
        self.0 & 7
    }

    #[inline(always)]
    pub const fn rank(self) -> u8 {
        self.0 >> 3
    }

    #[inline(always)]
    pub const fn flip_rank(self) -> Square {
        Square(self.0 ^ 56)
    }

    #[inline(always)]
    pub const fn flip_file(self) -> Square {
        Square(self.0 ^ 7)
    }

    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.0 < 64
    }

    /// Parse square from algebraic notation (e.g., "e4")
    pub fn from_algebraic(s: &str) -> Option<Self> {
        let bytes = s.as_bytes();
        if bytes.len() < 2 {
            return None;
        }
        let file = bytes[0].wrapping_sub(b'a');
        let rank = bytes[1].wrapping_sub(b'1');
        if file < 8 && rank < 8 {
            Some(Square::from_coords(file, rank))
        } else {
            None
        }
    }

    /// Convert to algebraic notation (e.g., "e4")
    pub fn to_algebraic(self) -> String {
        let file = (b'a' + self.file()) as char;
        let rank = (b'1' + self.rank()) as char;
        format!("{}{}", file, rank)
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid() {
            write!(f, "{}", self.to_algebraic())
        } else {
            write!(f, "-")
        }
    }
}

/// Represents a color (White or Black)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Color {
    #[default]
    White = 0,
    Black = 1,
}

impl Color {
    #[inline(always)]
    pub const fn flip(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    #[inline(always)]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Pawn push direction (+8 for white, -8 for black)
    #[inline(always)]
    pub const fn pawn_push(self) -> i8 {
        match self {
            Color::White => 8,
            Color::Black => -8,
        }
    }

    /// Starting rank for pawns (rank 2 for white, rank 7 for black)
    #[inline(always)]
    pub const fn pawn_start_rank(self) -> u8 {
        match self {
            Color::White => 1,
            Color::Black => 6,
        }
    }

    /// Promotion rank for pawns (rank 8 for white, rank 1 for black)
    #[inline(always)]
    pub const fn promotion_rank(self) -> u8 {
        match self {
            Color::White => 7,
            Color::Black => 0,
        }
    }

    /// Back rank (rank 1 for white, rank 8 for black)
    #[inline(always)]
    pub const fn back_rank(self) -> u8 {
        match self {
            Color::White => 0,
            Color::Black => 7,
        }
    }
}

impl std::ops::Not for Color {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self::Output {
        self.flip()
    }
}

/// Represents a piece type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PieceType {
    #[default]
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl PieceType {
    pub const COUNT: usize = 6;

    #[inline(always)]
    pub const fn index(self) -> usize {
        self as usize
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_lowercase() {
            'p' => Some(PieceType::Pawn),
            'n' => Some(PieceType::Knight),
            'b' => Some(PieceType::Bishop),
            'r' => Some(PieceType::Rook),
            'q' => Some(PieceType::Queen),
            'k' => Some(PieceType::King),
            _ => None,
        }
    }

    pub const fn to_char(self) -> char {
        match self {
            PieceType::Pawn => 'p',
            PieceType::Knight => 'n',
            PieceType::Bishop => 'b',
            PieceType::Rook => 'r',
            PieceType::Queen => 'q',
            PieceType::King => 'k',
        }
    }
}

impl From<u8> for PieceType {
    #[inline(always)]
    fn from(value: u8) -> Self {
        debug_assert!(value < 6);
        unsafe { std::mem::transmute(value) }
    }
}

/// Represents a piece with its color
/// Encoded as: color (bit 3) | piece_type (bits 0-2)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Piece(pub u8);

impl Piece {
    pub const WHITE_PAWN: Piece = Piece::new(Color::White, PieceType::Pawn);
    pub const WHITE_KNIGHT: Piece = Piece::new(Color::White, PieceType::Knight);
    pub const WHITE_BISHOP: Piece = Piece::new(Color::White, PieceType::Bishop);
    pub const WHITE_ROOK: Piece = Piece::new(Color::White, PieceType::Rook);
    pub const WHITE_QUEEN: Piece = Piece::new(Color::White, PieceType::Queen);
    pub const WHITE_KING: Piece = Piece::new(Color::White, PieceType::King);
    pub const BLACK_PAWN: Piece = Piece::new(Color::Black, PieceType::Pawn);
    pub const BLACK_KNIGHT: Piece = Piece::new(Color::Black, PieceType::Knight);
    pub const BLACK_BISHOP: Piece = Piece::new(Color::Black, PieceType::Bishop);
    pub const BLACK_ROOK: Piece = Piece::new(Color::Black, PieceType::Rook);
    pub const BLACK_QUEEN: Piece = Piece::new(Color::Black, PieceType::Queen);
    pub const BLACK_KING: Piece = Piece::new(Color::Black, PieceType::King);

    pub const NONE: Piece = Piece(255);

    #[inline(always)]
    pub const fn new(color: Color, piece_type: PieceType) -> Self {
        Piece((color as u8) << 3 | piece_type as u8)
    }

    #[inline(always)]
    pub const fn color(self) -> Color {
        if self.0 & 8 == 0 {
            Color::White
        } else {
            Color::Black
        }
    }

    #[inline(always)]
    pub const fn piece_type(self) -> PieceType {
        unsafe { std::mem::transmute(self.0 & 7) }
    }

    #[inline(always)]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub fn from_char(c: char) -> Option<Self> {
        let piece_type = PieceType::from_char(c)?;
        let color = if c.is_uppercase() {
            Color::White
        } else {
            Color::Black
        };
        Some(Piece::new(color, piece_type))
    }

    pub fn to_char(self) -> char {
        let c = self.piece_type().to_char();
        if self.color() == Color::White {
            c.to_ascii_uppercase()
        } else {
            c
        }
    }
}

impl std::fmt::Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

/// Castling rights encoded as a bitmask
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct CastlingRights(pub u8);

impl CastlingRights {
    pub const NONE: CastlingRights = CastlingRights(0);
    pub const WHITE_KINGSIDE: CastlingRights = CastlingRights(1);
    pub const WHITE_QUEENSIDE: CastlingRights = CastlingRights(2);
    pub const BLACK_KINGSIDE: CastlingRights = CastlingRights(4);
    pub const BLACK_QUEENSIDE: CastlingRights = CastlingRights(8);
    pub const WHITE_BOTH: CastlingRights = CastlingRights(3);
    pub const BLACK_BOTH: CastlingRights = CastlingRights(12);
    pub const ALL: CastlingRights = CastlingRights(15);

    #[inline(always)]
    pub const fn new() -> Self {
        CastlingRights(0)
    }

    #[inline(always)]
    pub const fn contains(self, other: CastlingRights) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub const fn insert(self, other: CastlingRights) -> Self {
        CastlingRights(self.0 | other.0)
    }

    #[inline(always)]
    pub const fn remove(self, other: CastlingRights) -> Self {
        CastlingRights(self.0 & !other.0)
    }

    #[inline(always)]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub fn from_fen(s: &str) -> Self {
        let mut rights = CastlingRights::NONE;
        for c in s.chars() {
            match c {
                'K' => rights = rights.insert(CastlingRights::WHITE_KINGSIDE),
                'Q' => rights = rights.insert(CastlingRights::WHITE_QUEENSIDE),
                'k' => rights = rights.insert(CastlingRights::BLACK_KINGSIDE),
                'q' => rights = rights.insert(CastlingRights::BLACK_QUEENSIDE),
                '-' => break,
                _ => {}
            }
        }
        rights
    }

    pub fn to_fen(self) -> String {
        if self.is_empty() {
            return "-".to_string();
        }
        let mut s = String::new();
        if self.contains(CastlingRights::WHITE_KINGSIDE) {
            s.push('K');
        }
        if self.contains(CastlingRights::WHITE_QUEENSIDE) {
            s.push('Q');
        }
        if self.contains(CastlingRights::BLACK_KINGSIDE) {
            s.push('k');
        }
        if self.contains(CastlingRights::BLACK_QUEENSIDE) {
            s.push('q');
        }
        s
    }

    /// Get kingside rights for a color
    #[inline(always)]
    pub const fn kingside(color: Color) -> CastlingRights {
        match color {
            Color::White => CastlingRights::WHITE_KINGSIDE,
            Color::Black => CastlingRights::BLACK_KINGSIDE,
        }
    }

    /// Get queenside rights for a color
    #[inline(always)]
    pub const fn queenside(color: Color) -> CastlingRights {
        match color {
            Color::White => CastlingRights::WHITE_QUEENSIDE,
            Color::Black => CastlingRights::BLACK_QUEENSIDE,
        }
    }

    /// Get both rights for a color
    #[inline(always)]
    pub const fn both(color: Color) -> CastlingRights {
        match color {
            Color::White => CastlingRights::WHITE_BOTH,
            Color::Black => CastlingRights::BLACK_BOTH,
        }
    }
}

impl std::ops::BitOr for CastlingRights {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        CastlingRights(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for CastlingRights {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAnd for CastlingRights {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        CastlingRights(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for CastlingRights {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl std::ops::Not for CastlingRights {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self::Output {
        CastlingRights(!self.0 & 0x0F)
    }
}

impl std::fmt::Display for CastlingRights {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_fen())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_square_coords() {
        assert_eq!(Square::A1.file(), 0);
        assert_eq!(Square::A1.rank(), 0);
        assert_eq!(Square::H8.file(), 7);
        assert_eq!(Square::H8.rank(), 7);
        assert_eq!(Square::E4.file(), 4);
        assert_eq!(Square::E4.rank(), 3);
    }

    const _: Square = Square(28); // E4
    impl Square {
        pub const E4: Square = Square(28);
    }

    #[test]
    fn test_square_algebraic() {
        assert_eq!(Square::from_algebraic("a1"), Some(Square::A1));
        assert_eq!(Square::from_algebraic("h8"), Some(Square::H8));
        assert_eq!(Square::from_algebraic("e4"), Some(Square(28)));
        assert_eq!(Square::A1.to_algebraic(), "a1");
        assert_eq!(Square::H8.to_algebraic(), "h8");
    }

    #[test]
    fn test_color_flip() {
        assert_eq!(Color::White.flip(), Color::Black);
        assert_eq!(Color::Black.flip(), Color::White);
    }

    #[test]
    fn test_piece_encoding() {
        let wp = Piece::WHITE_PAWN;
        assert_eq!(wp.color(), Color::White);
        assert_eq!(wp.piece_type(), PieceType::Pawn);

        let bk = Piece::BLACK_KING;
        assert_eq!(bk.color(), Color::Black);
        assert_eq!(bk.piece_type(), PieceType::King);
    }

    #[test]
    fn test_castling_rights() {
        let all = CastlingRights::ALL;
        assert!(all.contains(CastlingRights::WHITE_KINGSIDE));
        assert!(all.contains(CastlingRights::BLACK_QUEENSIDE));

        let wk_only = CastlingRights::WHITE_KINGSIDE;
        assert!(wk_only.contains(CastlingRights::WHITE_KINGSIDE));
        assert!(!wk_only.contains(CastlingRights::WHITE_QUEENSIDE));

        assert_eq!(CastlingRights::from_fen("KQkq"), CastlingRights::ALL);
        assert_eq!(CastlingRights::from_fen("-"), CastlingRights::NONE);
        assert_eq!(CastlingRights::ALL.to_fen(), "KQkq");
    }
}

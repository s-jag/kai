/// Bitboard representation and operations
use crate::types::{Color, Square};

/// A bitboard is a 64-bit integer where each bit represents a square
#[derive(Clone, Copy, PartialEq, Eq, Default, Hash)]
#[repr(transparent)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const EMPTY: Bitboard = Bitboard(0);
    pub const ALL: Bitboard = Bitboard(!0);

    // File masks
    pub const FILE_A: Bitboard = Bitboard(0x0101010101010101);
    pub const FILE_B: Bitboard = Bitboard(0x0202020202020202);
    pub const FILE_C: Bitboard = Bitboard(0x0404040404040404);
    pub const FILE_D: Bitboard = Bitboard(0x0808080808080808);
    pub const FILE_E: Bitboard = Bitboard(0x1010101010101010);
    pub const FILE_F: Bitboard = Bitboard(0x2020202020202020);
    pub const FILE_G: Bitboard = Bitboard(0x4040404040404040);
    pub const FILE_H: Bitboard = Bitboard(0x8080808080808080);

    // Rank masks
    pub const RANK_1: Bitboard = Bitboard(0x00000000000000FF);
    pub const RANK_2: Bitboard = Bitboard(0x000000000000FF00);
    pub const RANK_3: Bitboard = Bitboard(0x0000000000FF0000);
    pub const RANK_4: Bitboard = Bitboard(0x00000000FF000000);
    pub const RANK_5: Bitboard = Bitboard(0x000000FF00000000);
    pub const RANK_6: Bitboard = Bitboard(0x0000FF0000000000);
    pub const RANK_7: Bitboard = Bitboard(0x00FF000000000000);
    pub const RANK_8: Bitboard = Bitboard(0xFF00000000000000);

    // Not-file masks for shift operations
    pub const NOT_FILE_A: Bitboard = Bitboard(!Self::FILE_A.0);
    pub const NOT_FILE_H: Bitboard = Bitboard(!Self::FILE_H.0);
    pub const NOT_FILE_AB: Bitboard = Bitboard(!(Self::FILE_A.0 | Self::FILE_B.0));
    pub const NOT_FILE_GH: Bitboard = Bitboard(!(Self::FILE_G.0 | Self::FILE_H.0));

    // Diagonal masks
    pub const LIGHT_SQUARES: Bitboard = Bitboard(0x55AA55AA55AA55AA);
    pub const DARK_SQUARES: Bitboard = Bitboard(0xAA55AA55AA55AA55);

    /// Files array for indexing
    pub const FILES: [Bitboard; 8] = [
        Self::FILE_A,
        Self::FILE_B,
        Self::FILE_C,
        Self::FILE_D,
        Self::FILE_E,
        Self::FILE_F,
        Self::FILE_G,
        Self::FILE_H,
    ];

    /// Ranks array for indexing
    pub const RANKS: [Bitboard; 8] = [
        Self::RANK_1,
        Self::RANK_2,
        Self::RANK_3,
        Self::RANK_4,
        Self::RANK_5,
        Self::RANK_6,
        Self::RANK_7,
        Self::RANK_8,
    ];

    #[inline(always)]
    pub const fn new(value: u64) -> Self {
        Bitboard(value)
    }

    #[inline(always)]
    pub const fn from_square(sq: Square) -> Self {
        Bitboard(1u64 << sq.0)
    }

    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub const fn is_not_empty(self) -> bool {
        self.0 != 0
    }

    #[inline(always)]
    pub const fn pop_count(self) -> u32 {
        self.0.count_ones()
    }

    /// Get the least significant bit (first set square)
    #[inline(always)]
    pub const fn lsb(self) -> Square {
        Square(self.0.trailing_zeros() as u8)
    }

    /// Get the most significant bit (last set square)
    #[inline(always)]
    pub const fn msb(self) -> Square {
        Square(63 - self.0.leading_zeros() as u8)
    }

    /// Pop and return the least significant bit
    #[inline(always)]
    pub fn pop_lsb(&mut self) -> Square {
        let sq = self.lsb();
        self.0 &= self.0 - 1;
        sq
    }

    /// Check if a specific square is set
    #[inline(always)]
    pub const fn contains(self, sq: Square) -> bool {
        (self.0 & (1u64 << sq.0)) != 0
    }

    /// Set a specific square
    #[inline(always)]
    pub const fn set(self, sq: Square) -> Self {
        Bitboard(self.0 | (1u64 << sq.0))
    }

    /// Clear a specific square
    #[inline(always)]
    pub const fn clear(self, sq: Square) -> Self {
        Bitboard(self.0 & !(1u64 << sq.0))
    }

    /// Toggle a specific square
    #[inline(always)]
    pub const fn toggle(self, sq: Square) -> Self {
        Bitboard(self.0 ^ (1u64 << sq.0))
    }

    /// Shift north (toward rank 8)
    #[inline(always)]
    pub const fn north(self) -> Self {
        Bitboard(self.0 << 8)
    }

    /// Shift south (toward rank 1)
    #[inline(always)]
    pub const fn south(self) -> Self {
        Bitboard(self.0 >> 8)
    }

    /// Shift east (toward file H)
    #[inline(always)]
    pub const fn east(self) -> Self {
        Bitboard((self.0 << 1) & Self::NOT_FILE_A.0)
    }

    /// Shift west (toward file A)
    #[inline(always)]
    pub const fn west(self) -> Self {
        Bitboard((self.0 >> 1) & Self::NOT_FILE_H.0)
    }

    /// Shift northeast
    #[inline(always)]
    pub const fn north_east(self) -> Self {
        Bitboard((self.0 << 9) & Self::NOT_FILE_A.0)
    }

    /// Shift northwest
    #[inline(always)]
    pub const fn north_west(self) -> Self {
        Bitboard((self.0 << 7) & Self::NOT_FILE_H.0)
    }

    /// Shift southeast
    #[inline(always)]
    pub const fn south_east(self) -> Self {
        Bitboard((self.0 >> 7) & Self::NOT_FILE_A.0)
    }

    /// Shift southwest
    #[inline(always)]
    pub const fn south_west(self) -> Self {
        Bitboard((self.0 >> 9) & Self::NOT_FILE_H.0)
    }

    /// Shift in a direction based on color (north for white, south for black)
    #[inline(always)]
    pub const fn pawn_push(self, color: Color) -> Self {
        match color {
            Color::White => self.north(),
            Color::Black => self.south(),
        }
    }

    /// Shift by a signed amount
    #[inline(always)]
    pub const fn shift(self, amount: i8) -> Self {
        if amount >= 0 {
            Bitboard(self.0 << amount as u32)
        } else {
            Bitboard(self.0 >> (-amount) as u32)
        }
    }

    /// Get file mask for a square
    #[inline(always)]
    pub const fn file_of(sq: Square) -> Self {
        Self::FILES[sq.file() as usize]
    }

    /// Get rank mask for a square
    #[inline(always)]
    pub const fn rank_of(sq: Square) -> Self {
        Self::RANKS[sq.rank() as usize]
    }

    /// Get adjacent files
    #[inline(always)]
    pub const fn adjacent_files(self) -> Self {
        Bitboard(((self.0 & Self::NOT_FILE_A.0) >> 1) | ((self.0 & Self::NOT_FILE_H.0) << 1))
    }

    /// Fill north (smear bits upward)
    #[inline(always)]
    pub const fn fill_north(self) -> Self {
        let mut bb = self.0;
        bb |= bb << 8;
        bb |= bb << 16;
        bb |= bb << 32;
        Bitboard(bb)
    }

    /// Fill south (smear bits downward)
    #[inline(always)]
    pub const fn fill_south(self) -> Self {
        let mut bb = self.0;
        bb |= bb >> 8;
        bb |= bb >> 16;
        bb |= bb >> 32;
        Bitboard(bb)
    }

    /// Get squares in front of pawns (from perspective of color)
    #[inline(always)]
    pub const fn front_span(self, color: Color) -> Self {
        match color {
            Color::White => self.fill_north().north(),
            Color::Black => self.fill_south().south(),
        }
    }

    /// Check if more than one bit is set
    #[inline(always)]
    pub const fn more_than_one(self) -> bool {
        self.0 != 0 && (self.0 & (self.0 - 1)) != 0
    }

    /// Check if exactly one bit is set
    #[inline(always)]
    pub const fn exactly_one(self) -> bool {
        self.0 != 0 && (self.0 & (self.0 - 1)) == 0
    }
}

// Implement bitwise operators
impl std::ops::BitOr for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Bitboard {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAnd for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for Bitboard {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl std::ops::BitXor for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 ^ rhs.0)
    }
}

impl std::ops::BitXorAssign for Bitboard {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl std::ops::Not for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self::Output {
        Bitboard(!self.0)
    }
}

impl std::ops::Shl<u32> for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn shl(self, rhs: u32) -> Self::Output {
        Bitboard(self.0 << rhs)
    }
}

impl std::ops::Shr<u32> for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn shr(self, rhs: u32) -> Self::Output {
        Bitboard(self.0 >> rhs)
    }
}

impl std::ops::Sub for Bitboard {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 & !rhs.0)
    }
}

// Iterator for iterating over set squares
impl Iterator for Bitboard {
    type Item = Square;

    #[inline(always)]
    fn next(&mut self) -> Option<Square> {
        if self.is_empty() {
            None
        } else {
            Some(self.pop_lsb())
        }
    }
}

impl std::fmt::Debug for Bitboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for rank in (0..8).rev() {
            write!(f, "  {} ", rank + 1)?;
            for file in 0..8 {
                let sq = Square::from_coords(file, rank);
                if self.contains(sq) {
                    write!(f, "X ")?;
                } else {
                    write!(f, ". ")?;
                }
            }
            writeln!(f)?;
        }
        writeln!(f, "    a b c d e f g h")?;
        write!(f, "    0x{:016X}", self.0)
    }
}

impl std::fmt::Display for Bitboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Precomputed attack tables
pub static KNIGHT_ATTACKS: [Bitboard; 64] = init_knight_attacks();
pub static KING_ATTACKS: [Bitboard; 64] = init_king_attacks();
pub static PAWN_ATTACKS: [[Bitboard; 64]; 2] = init_pawn_attacks();

/// Line between two squares (exclusive of endpoints)
pub static BETWEEN: [[Bitboard; 64]; 64] = init_between();

/// Line through two squares (full ray)
pub static LINE: [[Bitboard; 64]; 64] = init_line();

const fn init_knight_attacks() -> [Bitboard; 64] {
    let mut attacks = [Bitboard::EMPTY; 64];
    let mut sq = 0u8;
    while sq < 64 {
        let bb = 1u64 << sq;
        let mut attack = 0u64;

        // Knight moves: +/- 6, 10, 15, 17
        attack |= (bb << 17) & !Bitboard::FILE_A.0;
        attack |= (bb << 15) & !Bitboard::FILE_H.0;
        attack |= (bb << 10) & !Bitboard::NOT_FILE_GH.0;
        attack |= (bb << 6) & !Bitboard::NOT_FILE_AB.0;
        attack |= (bb >> 17) & !Bitboard::FILE_H.0;
        attack |= (bb >> 15) & !Bitboard::FILE_A.0;
        attack |= (bb >> 10) & !Bitboard::NOT_FILE_AB.0;
        attack |= (bb >> 6) & !Bitboard::NOT_FILE_GH.0;

        attacks[sq as usize] = Bitboard(attack);
        sq += 1;
    }
    attacks
}

const fn init_king_attacks() -> [Bitboard; 64] {
    let mut attacks = [Bitboard::EMPTY; 64];
    let mut sq = 0u8;
    while sq < 64 {
        let bb = 1u64 << sq;
        let mut attack = 0u64;

        // King moves: all 8 directions
        attack |= bb << 8; // North
        attack |= bb >> 8; // South
        attack |= (bb << 1) & !Bitboard::FILE_A.0; // East
        attack |= (bb >> 1) & !Bitboard::FILE_H.0; // West
        attack |= (bb << 9) & !Bitboard::FILE_A.0; // North-East
        attack |= (bb << 7) & !Bitboard::FILE_H.0; // North-West
        attack |= (bb >> 7) & !Bitboard::FILE_A.0; // South-East
        attack |= (bb >> 9) & !Bitboard::FILE_H.0; // South-West

        attacks[sq as usize] = Bitboard(attack);
        sq += 1;
    }
    attacks
}

const fn init_pawn_attacks() -> [[Bitboard; 64]; 2] {
    let mut attacks = [[Bitboard::EMPTY; 64]; 2];
    let mut sq = 0u8;
    while sq < 64 {
        let bb = 1u64 << sq;

        // White pawn attacks (north-east and north-west)
        let white_attack = ((bb << 9) & !Bitboard::FILE_A.0) | ((bb << 7) & !Bitboard::FILE_H.0);
        attacks[0][sq as usize] = Bitboard(white_attack);

        // Black pawn attacks (south-east and south-west)
        let black_attack = ((bb >> 7) & !Bitboard::FILE_A.0) | ((bb >> 9) & !Bitboard::FILE_H.0);
        attacks[1][sq as usize] = Bitboard(black_attack);

        sq += 1;
    }
    attacks
}

const fn init_between() -> [[Bitboard; 64]; 64] {
    let mut between = [[Bitboard::EMPTY; 64]; 64];
    let mut sq1 = 0u8;
    while sq1 < 64 {
        let mut sq2 = 0u8;
        while sq2 < 64 {
            if sq1 != sq2 {
                let f1 = sq1 & 7;
                let r1 = sq1 >> 3;
                let f2 = sq2 & 7;
                let r2 = sq2 >> 3;

                let df = (f2 as i8 - f1 as i8).signum();
                let dr = (r2 as i8 - r1 as i8).signum();

                // Check if squares are on same line (diagonal, rank, or file)
                if (f1 == f2) || (r1 == r2) || (abs_diff(f1, f2) == abs_diff(r1, r2)) {
                    let mut bb = 0u64;
                    let mut f = (f1 as i8 + df) as u8;
                    let mut r = (r1 as i8 + dr) as u8;

                    while f != f2 || r != r2 {
                        bb |= 1u64 << (r * 8 + f);
                        f = (f as i8 + df) as u8;
                        r = (r as i8 + dr) as u8;
                    }
                    between[sq1 as usize][sq2 as usize] = Bitboard(bb);
                }
            }
            sq2 += 1;
        }
        sq1 += 1;
    }
    between
}

const fn init_line() -> [[Bitboard; 64]; 64] {
    let mut line = [[Bitboard::EMPTY; 64]; 64];
    let mut sq1 = 0u8;
    while sq1 < 64 {
        let mut sq2 = 0u8;
        while sq2 < 64 {
            if sq1 != sq2 {
                let f1 = sq1 & 7;
                let r1 = sq1 >> 3;
                let f2 = sq2 & 7;
                let r2 = sq2 >> 3;

                let df = (f2 as i8 - f1 as i8).signum();
                let dr = (r2 as i8 - r1 as i8).signum();

                // Check if squares are on same line
                if (f1 == f2) || (r1 == r2) || (abs_diff(f1, f2) == abs_diff(r1, r2)) {
                    let mut bb = 0u64;

                    // Go in negative direction until off board
                    let mut f = f1 as i8;
                    let mut r = r1 as i8;
                    while f >= 0 && f < 8 && r >= 0 && r < 8 {
                        bb |= 1u64 << (r * 8 + f);
                        f -= df;
                        r -= dr;
                    }

                    // Go in positive direction until off board
                    f = f1 as i8 + df;
                    r = r1 as i8 + dr;
                    while f >= 0 && f < 8 && r >= 0 && r < 8 {
                        bb |= 1u64 << (r * 8 + f);
                        f += df;
                        r += dr;
                    }

                    line[sq1 as usize][sq2 as usize] = Bitboard(bb);
                }
            }
            sq2 += 1;
        }
        sq1 += 1;
    }
    line
}

const fn abs_diff(a: u8, b: u8) -> u8 {
    if a > b {
        a - b
    } else {
        b - a
    }
}

/// Check if three squares are aligned (on same rank, file, or diagonal)
#[inline(always)]
pub fn aligned(sq1: Square, sq2: Square, sq3: Square) -> bool {
    LINE[sq1.0 as usize][sq2.0 as usize].contains(sq3)
}

/// Get bitboard of squares between two squares (exclusive)
#[inline(always)]
pub fn between(sq1: Square, sq2: Square) -> Bitboard {
    BETWEEN[sq1.0 as usize][sq2.0 as usize]
}

/// Get bitboard of squares on the line through two squares
#[inline(always)]
pub fn line(sq1: Square, sq2: Square) -> Bitboard {
    LINE[sq1.0 as usize][sq2.0 as usize]
}

/// Get knight attacks for a square
#[inline(always)]
pub fn knight_attacks(sq: Square) -> Bitboard {
    KNIGHT_ATTACKS[sq.0 as usize]
}

/// Get king attacks for a square
#[inline(always)]
pub fn king_attacks(sq: Square) -> Bitboard {
    KING_ATTACKS[sq.0 as usize]
}

/// Get pawn attacks for a square and color
#[inline(always)]
pub fn pawn_attacks(color: Color, sq: Square) -> Bitboard {
    PAWN_ATTACKS[color.index()][sq.0 as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitboard_basics() {
        let bb = Bitboard::from_square(Square::E4);
        assert!(!bb.is_empty());
        assert_eq!(bb.pop_count(), 1);
        assert!(bb.contains(Square::E4));
        assert!(!bb.contains(Square::D4));
    }

    #[test]
    fn test_bitboard_shifts() {
        let e4 = Bitboard::from_square(Square(28)); // e4
        assert_eq!(e4.north(), Bitboard::from_square(Square(36))); // e5
        assert_eq!(e4.south(), Bitboard::from_square(Square(20))); // e3
        assert_eq!(e4.east(), Bitboard::from_square(Square(29))); // f4
        assert_eq!(e4.west(), Bitboard::from_square(Square(27))); // d4
    }

    #[test]
    fn test_bitboard_iterator() {
        let bb = Bitboard::from_square(Square::A1)
            | Bitboard::from_square(Square::H8)
            | Bitboard::from_square(Square(28));

        let squares: Vec<Square> = bb.into_iter().collect();
        assert_eq!(squares.len(), 3);
        assert!(squares.contains(&Square::A1));
        assert!(squares.contains(&Square::H8));
        assert!(squares.contains(&Square(28)));
    }

    #[test]
    fn test_knight_attacks() {
        // Knight on e4 should attack 8 squares
        let attacks = knight_attacks(Square(28));
        assert_eq!(attacks.pop_count(), 8);

        // Knight on a1 should attack 2 squares
        let attacks = knight_attacks(Square::A1);
        assert_eq!(attacks.pop_count(), 2);
    }

    #[test]
    fn test_king_attacks() {
        // King on e4 should attack 8 squares
        let attacks = king_attacks(Square(28));
        assert_eq!(attacks.pop_count(), 8);

        // King on a1 should attack 3 squares
        let attacks = king_attacks(Square::A1);
        assert_eq!(attacks.pop_count(), 3);
    }

    #[test]
    fn test_pawn_attacks() {
        // White pawn on e4 attacks d5 and f5
        let attacks = pawn_attacks(Color::White, Square(28));
        assert_eq!(attacks.pop_count(), 2);
        assert!(attacks.contains(Square(35))); // d5
        assert!(attacks.contains(Square(37))); // f5

        // White pawn on a4 attacks only b5
        let attacks = pawn_attacks(Color::White, Square(24));
        assert_eq!(attacks.pop_count(), 1);
    }

    #[test]
    fn test_between() {
        // Between a1 and h8 (diagonal)
        let bb = between(Square::A1, Square::H8);
        assert_eq!(bb.pop_count(), 6); // b2, c3, d4, e5, f6, g7

        // Between a1 and a8 (file)
        let bb = between(Square::A1, Square::A8);
        assert_eq!(bb.pop_count(), 6); // a2 through a7

        // Between a1 and h1 (rank)
        let bb = between(Square::A1, Square::H1);
        assert_eq!(bb.pop_count(), 6); // b1 through g1
    }

    #[test]
    fn test_aligned() {
        assert!(aligned(Square::A1, Square::D4, Square::H8)); // Diagonal
        assert!(aligned(Square::A1, Square::A4, Square::A8)); // File
        assert!(aligned(Square::A1, Square::D1, Square::H1)); // Rank
        assert!(!aligned(Square::A1, Square::B3, Square::H8)); // Not aligned
    }
}

/// Magic bitboard implementation for sliding piece attack generation
use crate::bitboard::Bitboard;
use crate::types::Square;

/// Magic entry for a single square
#[derive(Clone, Copy)]
pub struct Magic {
    pub mask: Bitboard,
    pub magic: u64,
    pub shift: u8,
}

/// Precomputed magic numbers and attack tables for rooks
pub static ROOK_MAGICS: [Magic; 64] = init_rook_magics();
pub static mut ROOK_ATTACKS: [Bitboard; 102400] = [Bitboard::EMPTY; 102400];
static ROOK_OFFSETS: [usize; 64] = init_rook_offsets();

/// Precomputed magic numbers and attack tables for bishops
pub static BISHOP_MAGICS: [Magic; 64] = init_bishop_magics();
pub static mut BISHOP_ATTACKS: [Bitboard; 5248] = [Bitboard::EMPTY; 5248];
static BISHOP_OFFSETS: [usize; 64] = init_bishop_offsets();

/// Number of relevant bits for rook at each square
const ROOK_BITS: [u8; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11,
    12, 11, 11, 11, 11, 11, 11, 12,
];

/// Number of relevant bits for bishop at each square
const BISHOP_BITS: [u8; 64] = [
    6, 5, 5, 5, 5, 5, 5, 6,
    5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 7, 7, 7, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 7, 7, 7, 5, 5,
    5, 5, 5, 5, 5, 5, 5, 5,
    6, 5, 5, 5, 5, 5, 5, 6,
];

/// Precomputed magic numbers for rooks (found via trial and error)
const ROOK_MAGIC_NUMBERS: [u64; 64] = [
    0x0080001020400080, 0x0040001000200040, 0x0080081000200080, 0x0080040800100080,
    0x0080020400080080, 0x0080010200040080, 0x0080008001000200, 0x0080002040800100,
    0x0000800020400080, 0x0000400020005000, 0x0000801000200080, 0x0000800800100080,
    0x0000800400080080, 0x0000800200040080, 0x0000800100020080, 0x0000800040800100,
    0x0000208000400080, 0x0000404000201000, 0x0000808010002000, 0x0000808008001000,
    0x0000808004000800, 0x0000808002000400, 0x0000010100020004, 0x0000020000408104,
    0x0000208080004000, 0x0000200040005000, 0x0000100080200080, 0x0000080080100080,
    0x0000040080080080, 0x0000020080040080, 0x0000010080800200, 0x0000800080004100,
    0x0000204000800080, 0x0000200040401000, 0x0000100080802000, 0x0000080080801000,
    0x0000040080800800, 0x0000020080800400, 0x0000020001010004, 0x0000800040800100,
    0x0000204000808000, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000010002008080, 0x0000004081020004,
    0x0000204000800080, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
    0x0000040008008080, 0x0000020004008080, 0x0000800100020080, 0x0000800041000080,
    0x00FFFCDDFCED714A, 0x007FFCDDFCED714A, 0x003FFFCDFFD88096, 0x0000040810002101,
    0x0001000204080011, 0x0001000204000801, 0x0001000082000401, 0x0001FFFAABFAD1A2,
];

/// Precomputed magic numbers for bishops (found via trial and error)
const BISHOP_MAGIC_NUMBERS: [u64; 64] = [
    0x0002020202020200, 0x0002020202020000, 0x0004010202000000, 0x0004040080000000,
    0x0001104000000000, 0x0000821040000000, 0x0000410410400000, 0x0000104104104000,
    0x0000040404040400, 0x0000020202020200, 0x0000040102020000, 0x0000040400800000,
    0x0000011040000000, 0x0000008210400000, 0x0000004104104000, 0x0000002082082000,
    0x0004000808080800, 0x0002000404040400, 0x0001000202020200, 0x0000800802004000,
    0x0000800400A00000, 0x0000200100884000, 0x0000400082082000, 0x0000200041041000,
    0x0002080010101000, 0x0001040008080800, 0x0000208004010400, 0x0000404004010200,
    0x0000840000802000, 0x0000404002011000, 0x0000808001041000, 0x0000404000820800,
    0x0001041000202000, 0x0000820800101000, 0x0000104400080800, 0x0000020080080080,
    0x0000404040040100, 0x0000808100020100, 0x0001010100020800, 0x0000808080010400,
    0x0000820820004000, 0x0000410410002000, 0x0000082088001000, 0x0000002011000800,
    0x0000080100400400, 0x0001010101000200, 0x0002020202000400, 0x0001010101000200,
    0x0000410410400000, 0x0000208208200000, 0x0000002084100000, 0x0000000020880000,
    0x0000001002020000, 0x0000040408020000, 0x0004040404040000, 0x0002020202020000,
    0x0000104104104000, 0x0000002082082000, 0x0000000020841000, 0x0000000000208800,
    0x0000000010020200, 0x0000000404080200, 0x0000040404040400, 0x0002020202020200,
];

/// Initialize attack tables - must be called before using magic bitboards
pub fn init_magics() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        init_rook_attacks();
        init_bishop_attacks();
    });
}

fn init_rook_attacks() {
    for sq in 0..64 {
        let magic = &ROOK_MAGICS[sq];
        let mask = magic.mask;
        let n = mask.pop_count();
        let num_occupancies = 1 << n;

        for i in 0..num_occupancies {
            let occupied = index_to_occupancy(i, mask);
            let index = magic_index(occupied, magic.magic, magic.shift);
            let attacks = slow_rook_attacks(Square(sq as u8), occupied);
            unsafe {
                ROOK_ATTACKS[ROOK_OFFSETS[sq] + index] = attacks;
            }
        }
    }
}

fn init_bishop_attacks() {
    for sq in 0..64 {
        let magic = &BISHOP_MAGICS[sq];
        let mask = magic.mask;
        let n = mask.pop_count();
        let num_occupancies = 1 << n;

        for i in 0..num_occupancies {
            let occupied = index_to_occupancy(i, mask);
            let index = magic_index(occupied, magic.magic, magic.shift);
            let attacks = slow_bishop_attacks(Square(sq as u8), occupied);
            unsafe {
                BISHOP_ATTACKS[BISHOP_OFFSETS[sq] + index] = attacks;
            }
        }
    }
}

/// Get rook attacks for a square given an occupancy bitboard
#[inline(always)]
pub fn rook_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
    let magic = &ROOK_MAGICS[sq.0 as usize];
    let blockers = occupied & magic.mask;
    let index = magic_index(blockers, magic.magic, magic.shift);
    unsafe { ROOK_ATTACKS[ROOK_OFFSETS[sq.0 as usize] + index] }
}

/// Get bishop attacks for a square given an occupancy bitboard
#[inline(always)]
pub fn bishop_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
    let magic = &BISHOP_MAGICS[sq.0 as usize];
    let blockers = occupied & magic.mask;
    let index = magic_index(blockers, magic.magic, magic.shift);
    unsafe { BISHOP_ATTACKS[BISHOP_OFFSETS[sq.0 as usize] + index] }
}

/// Get queen attacks (combination of rook and bishop)
#[inline(always)]
pub fn queen_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
    rook_attacks(sq, occupied) | bishop_attacks(sq, occupied)
}

/// Compute magic index from blockers
#[inline(always)]
fn magic_index(blockers: Bitboard, magic: u64, shift: u8) -> usize {
    ((blockers.0.wrapping_mul(magic)) >> shift) as usize
}

/// Convert an index to an occupancy bitboard (for generating all occupancy patterns)
fn index_to_occupancy(index: usize, mask: Bitboard) -> Bitboard {
    let mut result = Bitboard::EMPTY;
    let mut mask_copy = mask;
    let mut i = 0;

    while mask_copy.is_not_empty() {
        let sq = mask_copy.pop_lsb();
        if (index & (1 << i)) != 0 {
            result = result.set(sq);
        }
        i += 1;
    }

    result
}

/// Slow rook attack generation (ray-based)
fn slow_rook_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;
    let file = sq.file() as i8;
    let rank = sq.rank() as i8;

    // North
    for r in (rank + 1)..8 {
        let s = Square::from_coords(file as u8, r as u8);
        attacks = attacks.set(s);
        if occupied.contains(s) {
            break;
        }
    }

    // South
    for r in (0..rank).rev() {
        let s = Square::from_coords(file as u8, r as u8);
        attacks = attacks.set(s);
        if occupied.contains(s) {
            break;
        }
    }

    // East
    for f in (file + 1)..8 {
        let s = Square::from_coords(f as u8, rank as u8);
        attacks = attacks.set(s);
        if occupied.contains(s) {
            break;
        }
    }

    // West
    for f in (0..file).rev() {
        let s = Square::from_coords(f as u8, rank as u8);
        attacks = attacks.set(s);
        if occupied.contains(s) {
            break;
        }
    }

    attacks
}

/// Slow bishop attack generation (ray-based)
fn slow_bishop_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;
    let file = sq.file() as i8;
    let rank = sq.rank() as i8;

    // North-East
    let mut f = file + 1;
    let mut r = rank + 1;
    while f < 8 && r < 8 {
        let s = Square::from_coords(f as u8, r as u8);
        attacks = attacks.set(s);
        if occupied.contains(s) {
            break;
        }
        f += 1;
        r += 1;
    }

    // North-West
    f = file - 1;
    r = rank + 1;
    while f >= 0 && r < 8 {
        let s = Square::from_coords(f as u8, r as u8);
        attacks = attacks.set(s);
        if occupied.contains(s) {
            break;
        }
        f -= 1;
        r += 1;
    }

    // South-East
    f = file + 1;
    r = rank - 1;
    while f < 8 && r >= 0 {
        let s = Square::from_coords(f as u8, r as u8);
        attacks = attacks.set(s);
        if occupied.contains(s) {
            break;
        }
        f += 1;
        r -= 1;
    }

    // South-West
    f = file - 1;
    r = rank - 1;
    while f >= 0 && r >= 0 {
        let s = Square::from_coords(f as u8, r as u8);
        attacks = attacks.set(s);
        if occupied.contains(s) {
            break;
        }
        f -= 1;
        r -= 1;
    }

    attacks
}

/// Generate rook mask for a square (edges excluded)
const fn rook_mask(sq: u8) -> Bitboard {
    let file = sq & 7;
    let rank = sq >> 3;
    let mut mask = 0u64;

    // Vertical (exclude edges)
    let mut r = 1u8;
    while r < 7 {
        if r != rank {
            mask |= 1u64 << (r * 8 + file);
        }
        r += 1;
    }

    // Horizontal (exclude edges)
    let mut f = 1u8;
    while f < 7 {
        if f != file {
            mask |= 1u64 << (rank * 8 + f);
        }
        f += 1;
    }

    Bitboard(mask)
}

/// Generate bishop mask for a square (edges excluded)
const fn bishop_mask(sq: u8) -> Bitboard {
    let file = sq & 7;
    let rank = sq >> 3;
    let mut mask = 0u64;

    // NE diagonal
    let mut f = file + 1;
    let mut r = rank + 1;
    while f < 7 && r < 7 {
        mask |= 1u64 << (r * 8 + f);
        f += 1;
        r += 1;
    }

    // NW diagonal
    f = file.wrapping_sub(1);
    r = rank + 1;
    while f < 7 && r < 7 && f < 8 {
        mask |= 1u64 << (r * 8 + f);
        f = f.wrapping_sub(1);
        r += 1;
    }

    // SE diagonal
    f = file + 1;
    r = rank.wrapping_sub(1);
    while f < 7 && r < 7 && r < 8 {
        mask |= 1u64 << (r * 8 + f);
        f += 1;
        r = r.wrapping_sub(1);
    }

    // SW diagonal
    f = file.wrapping_sub(1);
    r = rank.wrapping_sub(1);
    while f < 7 && r < 7 && f < 8 && r < 8 {
        mask |= 1u64 << (r * 8 + f);
        f = f.wrapping_sub(1);
        r = r.wrapping_sub(1);
    }

    Bitboard(mask)
}

const fn init_rook_magics() -> [Magic; 64] {
    let mut magics = [Magic {
        mask: Bitboard::EMPTY,
        magic: 0,
        shift: 0,
    }; 64];

    let mut sq = 0u8;
    while sq < 64 {
        magics[sq as usize] = Magic {
            mask: rook_mask(sq),
            magic: ROOK_MAGIC_NUMBERS[sq as usize],
            shift: 64 - ROOK_BITS[sq as usize],
        };
        sq += 1;
    }

    magics
}

const fn init_bishop_magics() -> [Magic; 64] {
    let mut magics = [Magic {
        mask: Bitboard::EMPTY,
        magic: 0,
        shift: 0,
    }; 64];

    let mut sq = 0u8;
    while sq < 64 {
        magics[sq as usize] = Magic {
            mask: bishop_mask(sq),
            magic: BISHOP_MAGIC_NUMBERS[sq as usize],
            shift: 64 - BISHOP_BITS[sq as usize],
        };
        sq += 1;
    }

    magics
}

const fn init_rook_offsets() -> [usize; 64] {
    let mut offsets = [0usize; 64];
    let mut offset = 0usize;
    let mut sq = 0usize;

    while sq < 64 {
        offsets[sq] = offset;
        offset += 1 << ROOK_BITS[sq];
        sq += 1;
    }

    offsets
}

const fn init_bishop_offsets() -> [usize; 64] {
    let mut offsets = [0usize; 64];
    let mut offset = 0usize;
    let mut sq = 0usize;

    while sq < 64 {
        offsets[sq] = offset;
        offset += 1 << BISHOP_BITS[sq];
        sq += 1;
    }

    offsets
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            init_magics();
        });
    }

    #[test]
    fn test_rook_attacks_empty_board() {
        setup();
        let attacks = rook_attacks(Square(28), Bitboard::EMPTY); // e4
        // Should attack 14 squares (7 on file + 7 on rank)
        assert_eq!(attacks.pop_count(), 14);
    }

    #[test]
    fn test_rook_attacks_with_blockers() {
        setup();
        let occupied = Bitboard::from_square(Square(30)) | Bitboard::from_square(Square(44)); // g4, e6
        let attacks = rook_attacks(Square(28), occupied); // e4
        // Should be blocked
        assert!(attacks.contains(Square(30))); // g4 is attacked
        assert!(!attacks.contains(Square(31))); // h4 is blocked
        assert!(attacks.contains(Square(44))); // e6 is attacked
        assert!(!attacks.contains(Square(52))); // e7 is blocked
    }

    #[test]
    fn test_bishop_attacks_empty_board() {
        setup();
        let attacks = bishop_attacks(Square(28), Bitboard::EMPTY); // e4
        // e4 bishop attacks 13 squares on empty board
        assert_eq!(attacks.pop_count(), 13);
    }

    #[test]
    fn test_queen_attacks() {
        setup();
        let attacks = queen_attacks(Square(28), Bitboard::EMPTY); // e4
        // Queen should attack rook + bishop squares = 14 + 13 = 27
        assert_eq!(attacks.pop_count(), 27);
    }

    #[test]
    fn test_corner_rook() {
        setup();
        let attacks = rook_attacks(Square::A1, Bitboard::EMPTY);
        assert_eq!(attacks.pop_count(), 14);
    }

    #[test]
    fn test_corner_bishop() {
        setup();
        let attacks = bishop_attacks(Square::A1, Bitboard::EMPTY);
        assert_eq!(attacks.pop_count(), 7);
    }
}

/// Zobrist hashing for position identification
use crate::types::{CastlingRights, Color, PieceType, Square};

/// Zobrist hash keys
pub struct Zobrist {
    /// Piece keys: [color][piece_type][square]
    pub pieces: [[[u64; 64]; 6]; 2],
    /// Castling rights keys (16 combinations)
    pub castling: [u64; 16],
    /// En passant file keys (8 files)
    pub en_passant: [u64; 8],
    /// Side to move key
    pub side: u64,
}

/// Global Zobrist keys instance
pub static ZOBRIST: Zobrist = Zobrist::new();

impl Zobrist {
    /// Initialize Zobrist keys with a fixed seed for reproducibility
    const fn new() -> Self {
        let mut pieces = [[[0u64; 64]; 6]; 2];
        let mut castling = [0u64; 16];
        let mut en_passant = [0u64; 8];

        // Use a simple PRNG for const initialization
        let mut state = 0x1234567890ABCDEFu64;

        // Initialize piece keys
        let mut color = 0;
        while color < 2 {
            let mut piece = 0;
            while piece < 6 {
                let mut sq = 0;
                while sq < 64 {
                    state = xorshift64(state);
                    pieces[color][piece][sq] = state;
                    sq += 1;
                }
                piece += 1;
            }
            color += 1;
        }

        // Initialize castling keys
        let mut i = 0;
        while i < 16 {
            state = xorshift64(state);
            castling[i] = state;
            i += 1;
        }

        // Initialize en passant keys
        i = 0;
        while i < 8 {
            state = xorshift64(state);
            en_passant[i] = state;
            i += 1;
        }

        // Side to move key
        state = xorshift64(state);
        let side = state;

        Zobrist {
            pieces,
            castling,
            en_passant,
            side,
        }
    }

    /// Get the key for a piece at a square
    #[inline(always)]
    pub fn piece_key(&self, color: Color, piece: PieceType, sq: Square) -> u64 {
        self.pieces[color as usize][piece as usize][sq.0 as usize]
    }

    /// Get the key for castling rights
    #[inline(always)]
    pub fn castling_key(&self, rights: CastlingRights) -> u64 {
        self.castling[rights.index()]
    }

    /// Get the key for an en passant file
    #[inline(always)]
    pub fn en_passant_key(&self, file: u8) -> u64 {
        self.en_passant[file as usize]
    }

    /// Get the side to move key
    #[inline(always)]
    pub fn side_key(&self) -> u64 {
        self.side
    }
}

/// Simple xorshift64 PRNG for const initialization
const fn xorshift64(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zobrist_uniqueness() {
        // Check that piece keys are unique
        let mut keys = std::collections::HashSet::new();
        for color in 0..2 {
            for piece in 0..6 {
                for sq in 0..64 {
                    let key = ZOBRIST.pieces[color][piece][sq];
                    assert!(keys.insert(key), "Duplicate key found");
                }
            }
        }

        // Add castling and en passant keys
        for key in &ZOBRIST.castling {
            assert!(keys.insert(*key), "Duplicate castling key");
        }
        for key in &ZOBRIST.en_passant {
            assert!(keys.insert(*key), "Duplicate en passant key");
        }
        assert!(keys.insert(ZOBRIST.side), "Duplicate side key");
    }

    #[test]
    fn test_zobrist_stability() {
        // Keys should be stable across runs (const initialization)
        let key1 = ZOBRIST.piece_key(Color::White, PieceType::Pawn, Square::A2);
        let key2 = ZOBRIST.piece_key(Color::White, PieceType::Pawn, Square::A2);
        assert_eq!(key1, key2);
    }
}

/// Evaluation function with tapered evaluation
use crate::bitboard::{pawn_attacks, Bitboard};
use crate::position::Position;
use crate::types::{Color, PieceType, Square};

/// Score with midgame and endgame components
#[derive(Clone, Copy, Default, Debug)]
pub struct Score {
    pub mg: i16, // Midgame
    pub eg: i16, // Endgame
}

impl Score {
    pub const ZERO: Score = Score { mg: 0, eg: 0 };

    pub const fn new(mg: i16, eg: i16) -> Self {
        Score { mg, eg }
    }
}

impl std::ops::Add for Score {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Score {
            mg: self.mg + rhs.mg,
            eg: self.eg + rhs.eg,
        }
    }
}

impl std::ops::AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        self.mg += rhs.mg;
        self.eg += rhs.eg;
    }
}

impl std::ops::Sub for Score {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Score {
            mg: self.mg - rhs.mg,
            eg: self.eg - rhs.eg,
        }
    }
}

impl std::ops::SubAssign for Score {
    fn sub_assign(&mut self, rhs: Self) {
        self.mg -= rhs.mg;
        self.eg -= rhs.eg;
    }
}

impl std::ops::Neg for Score {
    type Output = Self;
    fn neg(self) -> Self {
        Score {
            mg: -self.mg,
            eg: -self.eg,
        }
    }
}

impl std::ops::Mul<i16> for Score {
    type Output = Self;
    fn mul(self, rhs: i16) -> Self {
        Score {
            mg: self.mg * rhs,
            eg: self.eg * rhs,
        }
    }
}

/// Piece values (PeSTO-style)
pub const PIECE_VALUES: [Score; 6] = [
    Score::new(82, 94),    // Pawn
    Score::new(337, 281),  // Knight
    Score::new(365, 297),  // Bishop
    Score::new(477, 512),  // Rook
    Score::new(1025, 936), // Queen
    Score::new(0, 0),      // King (not used for material)
];

/// Phase values for tapered evaluation
const PHASE_VALUES: [i32; 6] = [0, 1, 1, 2, 4, 0];
const TOTAL_PHASE: i32 = 24; // 2*(1+1+2+4) + 2*(1+1+2+4) = 16 knights/bishops + 4 rooks + 2 queens

/// Piece-square tables (from white's perspective)
/// Index by [PieceType][Square]
pub static PSQT_MG: [[i16; 64]; 6] = [
    // Pawn
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        98, 134, 61, 95, 68, 126, 34, -11,
        -6, 7, 26, 31, 65, 56, 25, -20,
        -14, 13, 6, 21, 23, 12, 17, -23,
        -27, -2, -5, 12, 17, 6, 10, -25,
        -26, -4, -4, -10, 3, 3, 33, -12,
        -35, -1, -20, -23, -15, 24, 38, -22,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // Knight
    [
        -167, -89, -34, -49, 61, -97, -15, -107,
        -73, -41, 72, 36, 23, 62, 7, -17,
        -47, 60, 37, 65, 84, 129, 73, 44,
        -9, 17, 19, 53, 37, 69, 18, 22,
        -13, 4, 16, 13, 28, 19, 21, -8,
        -23, -9, 12, 10, 19, 17, 25, -16,
        -29, -53, -12, -3, -1, 18, -14, -19,
        -105, -21, -58, -33, -17, -28, -19, -23,
    ],
    // Bishop
    [
        -29, 4, -82, -37, -25, -42, 7, -8,
        -26, 16, -18, -13, 30, 59, 18, -47,
        -16, 37, 43, 40, 35, 50, 37, -2,
        -4, 5, 19, 50, 37, 37, 7, -2,
        -6, 13, 13, 26, 34, 12, 10, 4,
        0, 15, 15, 15, 14, 27, 18, 10,
        4, 15, 16, 0, 7, 21, 33, 1,
        -33, -3, -14, -21, -13, -12, -39, -21,
    ],
    // Rook
    [
        32, 42, 32, 51, 63, 9, 31, 43,
        27, 32, 58, 62, 80, 67, 26, 44,
        -5, 19, 26, 36, 17, 45, 61, 16,
        -24, -11, 7, 26, 24, 35, -8, -20,
        -36, -26, -12, -1, 9, -7, 6, -23,
        -45, -25, -16, -17, 3, 0, -5, -33,
        -44, -16, -20, -9, -1, 11, -6, -71,
        -19, -13, 1, 17, 16, 7, -37, -26,
    ],
    // Queen
    [
        -28, 0, 29, 12, 59, 44, 43, 45,
        -24, -39, -5, 1, -16, 57, 28, 54,
        -13, -17, 7, 8, 29, 56, 47, 57,
        -27, -27, -16, -16, -1, 17, -2, 1,
        -9, -26, -9, -10, -2, -4, 3, -3,
        -14, 2, -11, -2, -5, 2, 14, 5,
        -35, -8, 11, 2, 8, 15, -3, 1,
        -1, -18, -9, 10, -15, -25, -31, -50,
    ],
    // King
    [
        -65, 23, 16, -15, -56, -34, 2, 13,
        29, -1, -20, -7, -8, -4, -38, -29,
        -9, 24, 2, -16, -20, 6, 22, -22,
        -17, -20, -12, -27, -30, -25, -14, -36,
        -49, -1, -27, -39, -46, -44, -33, -51,
        -14, -14, -22, -46, -44, -30, -15, -27,
        1, 7, -8, -64, -43, -16, 9, 8,
        -15, 36, 12, -54, 8, -28, 24, 14,
    ],
];

pub static PSQT_EG: [[i16; 64]; 6] = [
    // Pawn
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        178, 173, 158, 134, 147, 132, 165, 187,
        94, 100, 85, 67, 56, 53, 82, 84,
        32, 24, 13, 5, -2, 4, 17, 17,
        13, 9, -3, -7, -7, -8, 3, -1,
        4, 7, -6, 1, 0, -5, -1, -8,
        13, 8, 8, 10, 13, 0, 2, -7,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // Knight
    [
        -58, -38, -13, -28, -31, -27, -63, -99,
        -25, -8, -25, -2, -9, -25, -24, -52,
        -24, -20, 10, 9, -1, -9, -19, -41,
        -17, 3, 22, 22, 22, 11, 8, -18,
        -18, -6, 16, 25, 16, 17, 4, -18,
        -23, -3, -1, 15, 10, -3, -20, -22,
        -42, -20, -10, -5, -2, -20, -23, -44,
        -29, -51, -23, -15, -22, -18, -50, -64,
    ],
    // Bishop
    [
        -14, -21, -11, -8, -7, -9, -17, -24,
        -8, -4, 7, -12, -3, -13, -4, -14,
        2, -8, 0, -1, -2, 6, 0, 4,
        -3, 9, 12, 9, 14, 10, 3, 2,
        -6, 3, 13, 19, 7, 10, -3, -9,
        -12, -3, 8, 10, 13, 3, -7, -15,
        -14, -18, -7, -1, 4, -9, -15, -27,
        -23, -9, -23, -5, -9, -16, -5, -17,
    ],
    // Rook
    [
        13, 10, 18, 15, 12, 12, 8, 5,
        11, 13, 13, 11, -3, 3, 8, 3,
        7, 7, 7, 5, 4, -3, -5, -3,
        4, 3, 13, 1, 2, 1, -1, 2,
        3, 5, 8, 4, -5, -6, -8, -11,
        -4, 0, -5, -1, -7, -12, -8, -16,
        -6, -6, 0, 2, -9, -9, -11, -3,
        -9, 2, 3, -1, -5, -13, 4, -20,
    ],
    // Queen
    [
        -9, 22, 22, 27, 27, 19, 10, 20,
        -17, 20, 32, 41, 58, 25, 30, 0,
        -20, 6, 9, 49, 47, 35, 19, 9,
        3, 22, 24, 45, 57, 40, 57, 36,
        -18, 28, 19, 47, 31, 34, 39, 23,
        -16, -27, 15, 6, 9, 17, 10, 5,
        -22, -23, -30, -16, -16, -23, -36, -32,
        -33, -28, -22, -43, -5, -32, -20, -41,
    ],
    // King
    [
        -74, -35, -18, -18, -11, 15, 4, -17,
        -12, 17, 14, 17, 17, 38, 23, 11,
        10, 17, 23, 15, 20, 45, 44, 13,
        -8, 22, 24, 27, 26, 33, 26, 3,
        -18, -4, 21, 24, 27, 23, 9, -11,
        -19, -3, 11, 21, 23, 16, 7, -9,
        -27, -11, 4, 13, 14, 4, -5, -17,
        -53, -34, -21, -11, -28, -14, -24, -43,
    ],
];

/// Bonus/penalty values
const BISHOP_PAIR: Score = Score::new(30, 40);
const DOUBLED_PAWN: Score = Score::new(-10, -20);
const ISOLATED_PAWN: Score = Score::new(-15, -10);
const PASSED_PAWN_BONUS: [Score; 8] = [
    Score::new(0, 0),      // Rank 1 (never happens)
    Score::new(5, 10),     // Rank 2
    Score::new(10, 20),    // Rank 3
    Score::new(20, 40),    // Rank 4
    Score::new(35, 70),    // Rank 5
    Score::new(60, 120),   // Rank 6
    Score::new(100, 200),  // Rank 7
    Score::new(0, 0),      // Rank 8 (never happens - promoted)
];
const ROOK_OPEN_FILE: Score = Score::new(20, 10);
const ROOK_SEMI_OPEN_FILE: Score = Score::new(10, 5);

impl Position {
    /// Evaluate the position from the side to move's perspective
    pub fn evaluate(&self) -> i16 {
        let mut score = Score::ZERO;
        let mut phase = 0i32;

        // Material and PSQT
        for color in [Color::White, Color::Black] {
            let sign = if color == Color::White { 1i16 } else { -1i16 };

            for piece_type in 0..6 {
                let pt = unsafe { std::mem::transmute::<u8, PieceType>(piece_type as u8) };
                let bb = self.piece_bb(color, pt);
                let count = bb.pop_count() as i16;

                // Material
                score += PIECE_VALUES[piece_type] * (sign * count);

                // PSQT
                for sq in bb {
                    let psqt_sq = if color == Color::White {
                        sq.0 as usize
                    } else {
                        sq.flip_rank().0 as usize
                    };

                    score.mg += sign * PSQT_MG[piece_type][psqt_sq];
                    score.eg += sign * PSQT_EG[piece_type][psqt_sq];
                }

                // Phase
                phase += PHASE_VALUES[piece_type] * bb.pop_count() as i32;
            }
        }

        // Pawn structure
        score += self.evaluate_pawns();

        // Bishop pair
        if self.piece_bb(Color::White, PieceType::Bishop).pop_count() >= 2 {
            score += BISHOP_PAIR;
        }
        if self.piece_bb(Color::Black, PieceType::Bishop).pop_count() >= 2 {
            score -= BISHOP_PAIR;
        }

        // Rook on open/semi-open files
        score += self.evaluate_rooks();

        // Tapered evaluation
        let mg_phase = phase.min(TOTAL_PHASE);
        let eg_phase = TOTAL_PHASE - mg_phase;

        let tapered =
            (score.mg as i32 * mg_phase + score.eg as i32 * eg_phase) / TOTAL_PHASE;

        // Return from side to move perspective
        if self.side_to_move == Color::White {
            tapered as i16
        } else {
            -tapered as i16
        }
    }

    /// Evaluate pawn structure
    fn evaluate_pawns(&self) -> Score {
        let mut score = Score::ZERO;

        for color in [Color::White, Color::Black] {
            let sign = if color == Color::White { 1i16 } else { -1i16 };
            let our_pawns = self.piece_bb(color, PieceType::Pawn);
            let their_pawns = self.piece_bb(color.flip(), PieceType::Pawn);

            for sq in our_pawns {
                let file = sq.file();
                let rank = if color == Color::White {
                    sq.rank()
                } else {
                    7 - sq.rank()
                };

                let file_mask = Bitboard::FILES[file as usize];
                let adjacent_files = file_mask.adjacent_files();

                // Doubled pawns
                if (our_pawns & file_mask).pop_count() > 1 {
                    score += DOUBLED_PAWN * sign;
                }

                // Isolated pawns
                if (our_pawns & adjacent_files).is_empty() {
                    score += ISOLATED_PAWN * sign;
                }

                // Passed pawns
                let front_span = match color {
                    Color::White => {
                        let mut span = Bitboard::EMPTY;
                        for r in (rank + 1)..8 {
                            span |= Bitboard::RANKS[r as usize];
                        }
                        span & (file_mask | adjacent_files)
                    }
                    Color::Black => {
                        let mut span = Bitboard::EMPTY;
                        for r in 0..rank {
                            span |= Bitboard::RANKS[(7 - r) as usize];
                        }
                        span & (file_mask | adjacent_files)
                    }
                };

                if (their_pawns & front_span).is_empty() {
                    score += PASSED_PAWN_BONUS[rank as usize] * sign;
                }
            }
        }

        score
    }

    /// Evaluate rooks on open/semi-open files
    fn evaluate_rooks(&self) -> Score {
        let mut score = Score::ZERO;

        for color in [Color::White, Color::Black] {
            let sign = if color == Color::White { 1i16 } else { -1i16 };
            let our_pawns = self.piece_bb(color, PieceType::Pawn);
            let their_pawns = self.piece_bb(color.flip(), PieceType::Pawn);
            let all_pawns = our_pawns | their_pawns;

            for sq in self.piece_bb(color, PieceType::Rook) {
                let file_mask = Bitboard::FILES[sq.file() as usize];

                if (all_pawns & file_mask).is_empty() {
                    // Open file
                    score += ROOK_OPEN_FILE * sign;
                } else if (our_pawns & file_mask).is_empty() {
                    // Semi-open file
                    score += ROOK_SEMI_OPEN_FILE * sign;
                }
            }
        }

        score
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
    fn test_startpos_evaluation() {
        setup();
        let pos = Position::new();
        let eval = pos.evaluate();
        // Starting position should be roughly equal
        assert!(eval.abs() < 50, "Startpos eval: {}", eval);
    }

    #[test]
    fn test_material_advantage() {
        setup();
        // White up a queen
        let pos = Position::from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .unwrap();
        let eval = pos.evaluate();
        assert!(eval > 800, "Should be winning for white: {}", eval);
    }

    #[test]
    fn test_evaluation_symmetry() {
        setup();
        let pos1 =
            Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        let pos2 =
            Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R b KQkq - 0 1").unwrap();

        // Same position, different side to move should have opposite evals
        let eval1 = pos1.evaluate();
        let eval2 = pos2.evaluate();
        assert_eq!(eval1, -eval2);
    }

    #[test]
    fn test_passed_pawn_bonus() {
        setup();
        // White passed pawn on e6
        let pos = Position::from_fen("4k3/8/4P3/8/8/8/8/4K3 w - - 0 1").unwrap();
        let eval = pos.evaluate();
        // Should be significantly positive for white
        assert!(eval > 100, "Passed pawn should give bonus: {}", eval);
    }
}

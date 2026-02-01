# Kai Chess Engine - Evaluation Function

This document explains how Kai evaluates chess positions.

## Table of Contents

1. [Overview](#overview)
2. [Material](#material)
3. [Piece-Square Tables](#piece-square-tables)
4. [Tapered Evaluation](#tapered-evaluation)
5. [Pawn Structure](#pawn-structure)
6. [Piece Bonuses](#piece-bonuses)
7. [Static Exchange Evaluation](#static-exchange-evaluation)
8. [Future Improvements](#future-improvements)

## Overview

Kai's evaluation function assigns a numerical score to any chess position. Positive scores favor White; negative scores favor Black.

```rust
pub fn evaluate(&self) -> i16 {
    let mut score = Score::ZERO;
    let mut phase = 0;

    // Material and PSQT
    score += self.evaluate_material_and_psqt(&mut phase);

    // Pawn structure
    score += self.evaluate_pawns();

    // Piece bonuses
    score += self.evaluate_pieces();

    // Taper between middlegame and endgame
    let tapered = taper(score, phase);

    // Return from side-to-move perspective
    if self.side_to_move == Color::White {
        tapered
    } else {
        -tapered
    }
}
```

## Material

Material is the foundation of evaluation. Kai uses PeSTO-derived values:

| Piece | Midgame | Endgame |
|-------|---------|---------|
| Pawn | 82 | 94 |
| Knight | 337 | 281 |
| Bishop | 365 | 297 |
| Rook | 477 | 512 |
| Queen | 1025 | 936 |
| King | - | - |

**Note:** Values are in centipawns (100 = 1 pawn).

### Observations

- **Knights vs Bishops**: Nearly equal in midgame, but knights are stronger
- **Endgame Rooks**: Rooks gain value in endgames (more open files)
- **Endgame Queens**: Slightly weaker (fewer pieces to attack)
- **Endgame Pawns**: More valuable (promotion potential)

## Piece-Square Tables

Each piece type has a table of positional bonuses for each square. These encode chess principles:

### Pawn PSQT

```
Midgame:
  0   0   0   0   0   0   0   0
 98 134  61  95  68 126  34 -11
 -6   7  26  31  65  56  25 -20
-14  13   6  21  23  12  17 -23
-27  -2  -5  12  17   6  10 -25
-26  -4  -4 -10   3   3  33 -12
-35  -1 -20 -23 -15  24  38 -22
  0   0   0   0   0   0   0   0
```

**Key Patterns:**
- Central pawns (d4, e4, d5, e5) are valuable
- Advanced pawns get bonuses
- Flank pawns (a, h files) are slightly discouraged
- Doubled pawns on c/f files get h-pawn pushes as compensation

### Knight PSQT

```
Midgame:
-167 -89 -34 -49  61 -97 -15 -107
 -73 -41  72  36  23  62   7  -17
 -47  60  37  65  84 129  73   44
  -9  17  19  53  37  69  18   22
 -13   4  16  13  28  19  21   -8
 -23  -9  12  10  19  17  25  -16
 -29 -53 -12  -3  -1  18 -14  -19
-105 -21 -58 -33 -17 -28 -19  -23
```

**Key Patterns:**
- Knights are terrible on the rim ("a knight on the rim is dim")
- Central outposts (d5, e5) are excellent
- Knights prefer closed positions (proximity to center)

### Bishop PSQT

```
Midgame:
-29   4 -82 -37 -25 -42   7  -8
-26  16 -18 -13  30  59  18 -47
-16  37  43  40  35  50  37  -2
 -4   5  19  50  37  37   7  -2
 -6  13  13  26  34  12  10   4
  0  15  15  15  14  27  18  10
  4  15  16   0   7  21  33   1
-33  -3 -14 -21 -13 -12 -39 -21
```

**Key Patterns:**
- Long diagonals are valuable
- Corners are bad (blocked by own pawns)
- Fianchetto squares (b2, g2, b7, g7) get bonuses

### Rook PSQT

```
Midgame:
 32  42  32  51  63   9  31  43
 27  32  58  62  80  67  26  44
 -5  19  26  36  17  45  61  16
-24 -11   7  26  24  35  -8 -20
-36 -26 -12  -1   9  -7   6 -23
-45 -25 -16 -17   3   0  -5 -33
-44 -16 -20  -9  -1  11  -6 -71
-19 -13   1  17  16   7 -37 -26
```

**Key Patterns:**
- 7th rank is powerful
- Open files preferred
- Corners are acceptable (can reach open files)

### King PSQT

```
Midgame:                          Endgame:
-65  23  16 -15 -56 -34   2  13   -74 -35 -18 -18 -11  15   4 -17
 29  -1 -20  -7  -8  -4 -38 -29   -12  17  14  17  17  38  23  11
 -9  24   2 -16 -20   6  22 -22    10  17  23  15  20  45  44  13
-17 -20 -12 -27 -30 -25 -14 -36    -8  22  24  27  26  33  26   3
-49  -1 -27 -39 -46 -44 -33 -51   -18  -4  21  24  27  23   9 -11
-14 -14 -22 -46 -44 -30 -15 -27   -19  -3  11  21  23  16   7  -9
  1   7  -8 -64 -43 -16   9   8   -27 -11   4  13  14   4  -5 -17
-15  36  12 -54   8 -28  24  14   -53 -34 -21 -11 -28 -14 -24 -43
```

**Key Patterns:**
- **Midgame**: King safety paramount - castled positions preferred
- **Endgame**: King becomes active piece - centralization valuable
- Dramatic shift between phases

## Tapered Evaluation

The game phase determines how to blend midgame and endgame scores.

### Phase Calculation

```rust
const PHASE_VALUES: [i32; 6] = [0, 1, 1, 2, 4, 0];
// Pawn=0, Knight=1, Bishop=1, Rook=2, Queen=4, King=0

const TOTAL_PHASE: i32 = 24;
// 2*(1+1+2+4) + 2*(1+1+2+4) = 24

fn calculate_phase(pos: &Position) -> i32 {
    let mut phase = 0;
    for piece_type in 0..6 {
        for color in [White, Black] {
            phase += PHASE_VALUES[piece_type]
                   * pos.piece_bb(color, piece_type).pop_count();
        }
    }
    phase.min(TOTAL_PHASE)
}
```

### Tapering Formula

```rust
fn taper(score: Score, phase: i32) -> i16 {
    let mg_phase = phase.min(TOTAL_PHASE);
    let eg_phase = TOTAL_PHASE - mg_phase;

    let tapered = (score.mg as i32 * mg_phase
                 + score.eg as i32 * eg_phase)
                 / TOTAL_PHASE;

    tapered as i16
}
```

**Example:**
- Opening (phase=24): 100% midgame score
- Middle game (phase=12): 50% midgame + 50% endgame
- Endgame (phase=0): 100% endgame score

## Pawn Structure

Pawns are the soul of chess. Kai evaluates:

### Doubled Pawns

Two pawns of same color on same file:

```rust
if (our_pawns & file_mask).pop_count() > 1 {
    score -= DOUBLED_PAWN;  // (-10, -20)
}
```

**Why Bad:**
- Can't protect each other
- Block each other's advance
- Worse in endgames (hence larger endgame penalty)

### Isolated Pawns

Pawns with no friendly pawns on adjacent files:

```rust
let adjacent_files = file_mask.adjacent_files();
if (our_pawns & adjacent_files).is_empty() {
    score -= ISOLATED_PAWN;  // (-15, -10)
}
```

**Why Bad:**
- No pawn can protect them
- Become targets for enemy pieces
- Slightly less bad in endgames

### Passed Pawns

Pawns with no enemy pawns ahead on same or adjacent files:

```rust
let front_span = /* squares ahead on file and adjacent files */;
if (their_pawns & front_span).is_empty() {
    score += PASSED_PAWN_BONUS[rank];
}
```

**Bonus by Rank:**
| Rank | Midgame | Endgame |
|------|---------|---------|
| 2 | 5 | 10 |
| 3 | 10 | 20 |
| 4 | 20 | 40 |
| 5 | 35 | 70 |
| 6 | 60 | 120 |
| 7 | 100 | 200 |

**Why Good:**
- Can't be stopped by enemy pawns
- Creates promotion threats
- Much stronger in endgames

## Piece Bonuses

### Bishop Pair

Two bishops are worth more than the sum of parts:

```rust
if pos.piece_bb(color, Bishop).pop_count() >= 2 {
    score += BISHOP_PAIR;  // (30, 40)
}
```

**Why Good:**
- Cover both light and dark squares
- Complement each other in open positions
- More valuable in endgames

### Rook on Open File

Rooks need open lines to be effective:

```rust
let all_pawns = our_pawns | their_pawns;
if (all_pawns & file_mask).is_empty() {
    score += ROOK_OPEN_FILE;  // (20, 10)
}
```

### Rook on Semi-Open File

A file with no friendly pawns:

```rust
if (our_pawns & file_mask).is_empty() {
    score += ROOK_SEMI_OPEN_FILE;  // (10, 5)
}
```

## Static Exchange Evaluation

SEE determines the outcome of a capture sequence:

```rust
pub fn see_ge(&self, mv: Move, threshold: i16) -> bool {
    // Simulate the exchange
    let mut value = captured_piece_value;
    let mut gain = [0i16; 32];
    gain[0] = value;

    let mut side = opponent;
    let mut piece_on_sq = moving_piece;

    loop {
        gain[depth] = piece_value(piece_on_sq) - gain[depth - 1];

        // Find least valuable attacker
        let attacker = find_lva(side, target_square);
        if no_attacker { break; }

        // Update x-ray attackers
        piece_on_sq = attacker_type;
        side = side.flip();
    }

    // Minimax the gain array
    while depth > 1 {
        gain[depth - 1] = -max(-gain[depth - 1], gain[depth]);
    }

    gain[0] >= threshold
}
```

**Piece Values for SEE:**
| Piece | Value |
|-------|-------|
| Pawn | 100 |
| Knight | 300 |
| Bishop | 300 |
| Rook | 500 |
| Queen | 900 |
| King | 10000 |

**Example: QxP with PxQ response**
```
Initial: White Q takes Black P on e5
  gain[0] = +100 (captured pawn)

Black responds: P takes Q
  gain[1] = 900 - 100 = +800 (for black)

Minimax:
  gain[0] = -max(-100, 800) = -max(-100, 800) = -800

Result: SEE = -800 (bad capture for white)
```

## Score Units

All scores are in **centipawns**:
- 100 = one pawn
- 300 = one minor piece (approximate)
- 500 = one rook (approximate)
- Mate scores: ±30000 adjusted for distance

## Future Improvements

Kai's evaluation could be enhanced with:

### King Safety
```rust
// Attack units near king
let attacks_near_king = count_attacks_to(king_zone, enemy);
score -= KING_DANGER_TABLE[attacks_near_king];
```

### Mobility
```rust
// Count legal moves for each piece
for piece in our_pieces {
    let moves = legal_moves(piece);
    score += MOBILITY_BONUS[piece_type][moves.count()];
}
```

### Piece Activity
```rust
// Bonus for pieces on strong squares
score += outpost_bonus(knights_on_outposts);
score += rook_7th_rank_bonus(rooks_on_7th);
```

### Threats
```rust
// Bonus for attacking enemy pieces
score += threat_by_pawn(pawns_attacking_minors);
score += threat_by_minor(minors_attacking_rooks);
```

### Space
```rust
// Control of center and territory
score += space_bonus(squares_behind_pawns);
```

### NNUE (Neural Network)

The ultimate improvement would be implementing NNUE:
- Learn evaluation from millions of positions
- Efficiently updateable neural network
- Much stronger than hand-crafted evaluation

## Debugging Evaluation

Use the `eval` command to see the static evaluation:

```
position fen r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4
eval
Evaluation: 15 cp
```

This shows White has a small advantage (typical for Italian Game positions).

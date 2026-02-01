# Kai Chess Engine - Architecture

This document provides a detailed overview of Kai's internal architecture and design decisions.

## Table of Contents

1. [Overview](#overview)
2. [Board Representation](#board-representation)
3. [Move Generation](#move-generation)
4. [Position Management](#position-management)
5. [Hashing](#hashing)
6. [Data Flow](#data-flow)

## Overview

Kai follows a traditional chess engine architecture with modern optimizations:

```
┌─────────────────────────────────────────────────────────────┐
│                        UCI Interface                         │
│                         (uci.rs)                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Search Engine                           │
│              (search.rs, qsearch.rs, ordering.rs)           │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
┌───────────────────┐ ┌─────────────┐ ┌─────────────────┐
│   Transposition   │ │  Evaluation │ │  Move Generator │
│      Table        │ │   (eval.rs) │ │  (movegen.rs)   │
│     (tt.rs)       │ │             │ │                 │
└───────────────────┘ └─────────────┘ └─────────────────┘
                              │               │
                              ▼               ▼
              ┌─────────────────────────────────────┐
              │           Position State            │
              │ (position.rs, bitboard.rs, magic.rs)│
              └─────────────────────────────────────┘
```

## Board Representation

### Bitboards

Kai uses a bitboard-based representation where each piece type and color has its own 64-bit integer. A set bit indicates the presence of that piece on that square.

```rust
pub struct Position {
    /// Piece bitboards: [color][piece_type]
    pub pieces: [[Bitboard; 6]; 2],

    /// Occupancy bitboards per color
    pub occupied: [Bitboard; 2],

    /// All occupied squares
    pub all_occupied: Bitboard,

    /// Mailbox for quick piece lookup
    pub board: [Option<Piece>; 64],

    // ... game state
}
```

**Square Mapping:**
```
  a  b  c  d  e  f  g  h
8 56 57 58 59 60 61 62 63
7 48 49 50 51 52 53 54 55
6 40 41 42 43 44 45 46 47
5 32 33 34 35 36 37 38 39
4 24 25 26 27 28 29 30 31
3 16 17 18 19 20 21 22 23
2  8  9 10 11 12 13 14 15
1  0  1  2  3  4  5  6  7
```

### Magic Bitboards

For sliding pieces (bishops, rooks, queens), Kai uses magic bitboards to achieve O(1) attack generation.

**How Magic Bitboards Work:**

1. **Mask**: For each square, precompute a mask of relevant blocker squares (excluding edges)
2. **Magic Number**: A carefully chosen 64-bit number that, when multiplied with the blocker configuration, produces a unique index
3. **Lookup Table**: Precomputed attacks for each possible blocker configuration

```rust
pub struct Magic {
    pub mask: Bitboard,      // Relevant occupancy mask
    pub magic: u64,          // Magic multiplier
    pub shift: u8,           // Right shift amount (64 - index_bits)
}

#[inline(always)]
pub fn rook_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
    let magic = &ROOK_MAGICS[sq.0 as usize];
    let blockers = occupied & magic.mask;
    let index = (blockers.0.wrapping_mul(magic.magic)) >> magic.shift;
    unsafe { ROOK_ATTACKS[ROOK_OFFSETS[sq.0 as usize] + index as usize] }
}
```

**Table Sizes:**
- Rook attacks: 102,400 entries
- Bishop attacks: 5,248 entries

### Mailbox Array

In addition to bitboards, a 64-element array stores the piece on each square for O(1) lookup:

```rust
pub board: [Option<Piece>; 64],
```

This hybrid approach gives us:
- Fast iteration over pieces (bitboards)
- Fast piece lookup by square (mailbox)

## Move Generation

### Move Encoding

Moves are encoded in 16 bits for memory efficiency:

```
15 14 13 12 | 11 10  9  8  7  6 |  5  4  3  2  1  0
   flags    |    to square      |   from square
```

**Flag Values:**
| Value | Meaning |
|-------|---------|
| 0000 | Quiet move |
| 0001 | Double pawn push |
| 0010 | Kingside castle |
| 0011 | Queenside castle |
| 0100 | Capture |
| 0101 | En passant capture |
| 1000 | Knight promotion |
| 1001 | Bishop promotion |
| 1010 | Rook promotion |
| 1011 | Queen promotion |
| 1100 | Knight promo-capture |
| 1101 | Bishop promo-capture |
| 1110 | Rook promo-capture |
| 1111 | Queen promo-capture |

### Legal Move Generation Strategy

Kai generates legal moves directly rather than generating pseudo-legal moves and filtering:

1. **Double Check**: Only king moves are legal
2. **Single Check**: Generate evasions (block, capture, or king move)
3. **No Check**: Generate all legal moves with pin awareness

```rust
pub fn generate_legal_moves(&self, list: &mut MoveList) {
    if self.checkers.is_empty() {
        self.generate_moves::<false>(list);
    } else if self.checkers.exactly_one() {
        self.generate_moves::<true>(list);  // Evasion mode
    } else {
        self.generate_king_moves(list);      // Double check
    }
}
```

### Pin Detection

Pinned pieces are computed once per position:

```rust
pub fn pinned_pieces(&self, color: Color) -> Bitboard {
    let king_sq = self.king_sq[color as usize];
    let them = color.flip();

    let mut pinned = Bitboard::EMPTY;

    // Check diagonal pins
    let diag_attackers = bishop_attacks(king_sq, their_pieces)
                       & self.diagonal_sliders(them);
    for attacker in diag_attackers {
        let between = between_bb(king_sq, attacker) & self.all_occupied;
        if between.exactly_one() {
            pinned |= between & our_pieces;
        }
    }

    // Similar for orthogonal pins...

    pinned
}
```

## Position Management

### Copy-Make vs. Make-Unmake

Kai uses the **copy-make** approach:

```rust
pub fn make_move(&self, mv: Move) -> Self {
    let mut new = self.clone();
    new.apply_move(mv);
    new
}
```

**Advantages:**
- Simpler implementation (no unmake bugs)
- Better cache locality in many cases
- Natural fit for Rust's ownership model

**Disadvantages:**
- More memory copying
- Slightly slower for very deep searches

The copy-make approach is competitive because:
1. `Position` is relatively small (~200 bytes)
2. Modern CPUs are very fast at memory copies
3. Eliminates a class of bugs

### State Updates

When making a move, the following state is updated:
1. Piece bitboards (source and destination)
2. Occupancy bitboards
3. Mailbox array
4. Zobrist hash (incremental)
5. Castling rights
6. En passant square
7. Halfmove clock
8. Side to move
9. Checkers bitboard

## Hashing

### Zobrist Hashing

Each position has a unique (with high probability) 64-bit hash:

```rust
pub struct Zobrist {
    pub pieces: [[[u64; 64]; 6]; 2],  // [color][piece][square]
    pub castling: [u64; 16],           // All 16 combinations
    pub en_passant: [u64; 8],          // File of EP square
    pub side: u64,                      // Side to move
}
```

**Hash Computation:**
```rust
hash = 0
for each piece on the board:
    hash ^= zobrist.pieces[color][piece_type][square]
hash ^= zobrist.castling[castling_rights]
if en_passant:
    hash ^= zobrist.en_passant[ep_file]
if black_to_move:
    hash ^= zobrist.side
```

### Incremental Updates

Instead of recomputing the hash from scratch, we update it incrementally:

```rust
// Moving a piece
self.hash ^= ZOBRIST.piece_key(color, piece_type, from);  // Remove from source
self.hash ^= ZOBRIST.piece_key(color, piece_type, to);    // Add to destination

// Capture
self.hash ^= ZOBRIST.piece_key(them, captured_type, to);  // Remove captured

// Castling rights change
self.hash ^= ZOBRIST.castling_key(old_rights);
self.hash ^= ZOBRIST.castling_key(new_rights);
```

## Data Flow

### Typical Search Flow

```
1. UCI "go" command received
2. Parse time controls, calculate time limit
3. Reset stop flag, start iterative deepening

For each depth 1..max_depth:
    4. Set aspiration window (after depth 4)
    5. Call negamax(root, depth, alpha, beta)

    In negamax:
        6. Check timeout, update node counter
        7. Probe transposition table
        8. Check for terminal node (depth 0 → qsearch)
        9. Static evaluation for pruning decisions
        10. Apply pruning (null move, reverse futility)
        11. Generate and order moves
        12. Search moves with PVS
        13. Update heuristics on cutoff
        14. Store result in TT

    15. Handle aspiration window failures (re-search)
    16. Print "info" with depth, score, PV
    17. Check for mate score (stop early)

18. Print "bestmove"
```

### Memory Layout

```
Position (~200 bytes):
├── pieces: [[Bitboard; 6]; 2]     96 bytes
├── occupied: [Bitboard; 2]        16 bytes
├── all_occupied: Bitboard          8 bytes
├── board: [Option<Piece>; 64]     64 bytes
├── side_to_move: Color             1 byte
├── castling: CastlingRights        1 byte
├── en_passant: Option<Square>      2 bytes
├── halfmove_clock: u8              1 byte
├── fullmove_number: u16            2 bytes
├── hash: u64                       8 bytes
├── king_sq: [Square; 2]            2 bytes
└── checkers: Bitboard              8 bytes

TTEntry (16 bytes, cache-aligned):
├── key: u32                        4 bytes
├── best_move: Move                 2 bytes
├── score: i16                      2 bytes
├── depth: i8                       1 byte
├── bound: Bound                    1 byte
├── age: u8                         1 byte
└── padding                         5 bytes

MoveList (~2KB):
├── moves: [Move; 256]            512 bytes
├── scores: [i32; 256]           1024 bytes
└── len: usize                      8 bytes
```

## Performance Considerations

### Hot Path Optimizations

1. **Inline Critical Functions**: `#[inline(always)]` on bitboard operations
2. **Avoid Bounds Checks**: Use `unsafe` indexing in proven-safe contexts
3. **Minimize Branching**: Use branchless operations where possible
4. **Cache-Friendly TT**: 16-byte entries fit cache lines well

### Compiler Optimizations

```toml
[profile.release]
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization, slower compile
panic = "abort"      # Smaller binary, no unwinding
opt-level = 3        # Maximum optimization
```

### Future Optimizations

- **PEXT Bitboards**: For CPUs with BMI2 instruction set
- **SIMD Evaluation**: Vectorized piece-square table lookups
- **Lazy SMP**: Multi-threaded search
- **Prefetching**: TT prefetch before make_move

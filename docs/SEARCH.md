# Kai Chess Engine - Search Algorithm

This document explains Kai's search algorithm in detail, including all pruning and reduction techniques.

## Table of Contents

1. [Overview](#overview)
2. [Iterative Deepening](#iterative-deepening)
3. [Alpha-Beta Pruning](#alpha-beta-pruning)
4. [Principal Variation Search](#principal-variation-search)
5. [Transposition Table](#transposition-table)
6. [Pruning Techniques](#pruning-techniques)
7. [Move Ordering](#move-ordering)
8. [Quiescence Search](#quiescence-search)
9. [Time Management](#time-management)

## Overview

Kai uses a **negamax** search with **alpha-beta pruning** as its core algorithm. This is enhanced with numerous techniques to reduce the search tree size while maintaining correctness.

```
Search Structure:
┌─────────────────────────────────────────┐
│          Iterative Deepening            │
│         (depth 1, 2, 3, ... n)          │
└─────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│           Aspiration Windows            │
│     (narrow α-β window, re-search)      │
└─────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│         Principal Variation Search      │
│   (full window first, null window rest) │
└─────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│              Negamax Search             │
│    (with pruning and reductions)        │
└─────────────────────────────────────────┘
                    │
              at depth 0
                    ▼
┌─────────────────────────────────────────┐
│           Quiescence Search             │
│        (captures and promotions)        │
└─────────────────────────────────────────┘
```

## Iterative Deepening

Instead of searching directly to a target depth, Kai searches progressively deeper:

```rust
for depth in 1..=max_depth {
    let score = self.negamax(depth, 0, alpha, beta, ...);
    // Use results for move ordering in next iteration
}
```

**Benefits:**
1. **Time Control**: Can stop anytime and return best move so far
2. **Move Ordering**: Previous iteration's best move searched first
3. **Aspiration Windows**: Previous score guides window bounds
4. **TT Population**: Shallower results populate TT for deeper search

## Alpha-Beta Pruning

The fundamental optimization over minimax. We maintain a window [α, β] and prune branches that can't affect the result.

```rust
fn negamax(&self, depth: i32, ply: i32, mut alpha: i16, beta: i16, ...) -> i16 {
    // ... initialization

    for mv in moves {
        let score = -child.negamax(depth - 1, ply + 1, -beta, -alpha, ...);

        if score >= beta {
            return beta;  // Beta cutoff - opponent won't allow this
        }
        if score > alpha {
            alpha = score;  // New best move found
        }
    }

    return alpha;
}
```

**Key Insight**: If we've found a move scoring β or higher, the opponent already has a better option earlier in the tree, so this branch is irrelevant.

## Principal Variation Search

PVS (also called NegaScout) optimizes alpha-beta by assuming the first move is best:

```rust
for i in 0..moves.len() {
    let mv = pick_move(&mut moves, i);
    let new_pos = self.make_move(mv);

    let score = if i == 0 {
        // Full window for first move
        -new_pos.negamax(depth - 1, ply + 1, -beta, -alpha, ...)
    } else {
        // Null window search
        let score = -new_pos.negamax(depth - 1, ply + 1, -alpha - 1, -alpha, ...);

        // Re-search with full window if null window fails high
        if score > alpha && score < beta {
            -new_pos.negamax(depth - 1, ply + 1, -beta, -alpha, ...)
        } else {
            score
        }
    };
}
```

**Why It Works**: If move ordering is good, subsequent moves rarely beat the first. The null-window search is faster (more cutoffs), and we only pay for re-search when needed.

## Transposition Table

The TT stores previously searched positions to avoid redundant work.

### Entry Structure

```rust
pub struct TTEntry {
    pub key: u32,        // Hash verification
    pub best_move: Move, // Best move found
    pub score: i16,      // Evaluation score
    pub depth: i8,       // Search depth
    pub bound: Bound,    // Type of bound
    pub age: u8,         // Search age
}

pub enum Bound {
    Exact,  // Score is exact (PV node)
    Lower,  // Score is lower bound (beta cutoff)
    Upper,  // Score is upper bound (fail low)
}
```

### Probing

```rust
if let Some(entry) = tt.probe(self.hash) {
    if entry.depth >= depth {
        match entry.bound {
            Bound::Exact => return entry.score,
            Bound::Lower if entry.score >= beta => return entry.score,
            Bound::Upper if entry.score <= alpha => return entry.score,
            _ => { /* Use TT move for ordering */ }
        }
    }
    tt_move = entry.best_move;
}
```

### Mate Score Adjustment

Mate scores are relative to the root, not the current node:

```rust
// When storing
fn score_to_tt(score: i16, ply: i32) -> i16 {
    if score >= MATE_SCORE - MAX_PLY {
        score + ply as i16  // Adjust for distance from root
    } else if score <= -MATE_SCORE + MAX_PLY {
        score - ply as i16
    } else {
        score
    }
}

// When retrieving
fn score_from_tt(score: i16, ply: i32) -> i16 {
    // Inverse adjustment
}
```

## Pruning Techniques

### Null Move Pruning

If our position is so good that passing gives us a beta cutoff, the position is likely won.

```rust
if !is_pv && !in_check && depth >= 3 && static_eval >= beta {
    // Don't null move with only pawns (zugzwang risk)
    if has_non_pawn_material {
        let r = 3 + depth / 4;  // Reduction
        let null_pos = self.make_null_move();
        let score = -null_pos.negamax(depth - 1 - r, ply + 1, -beta, -beta + 1, ...);

        if score >= beta {
            return score;  // Position is likely winning
        }
    }
}
```

**Conditions:**
- Not in PV (principal variation) nodes
- Not in check
- Sufficient depth
- Has non-pawn material (avoids zugzwang)

### Reverse Futility Pruning (Static Null Move Pruning)

If static evaluation is far above beta, prune without searching.

```rust
if !is_pv && !in_check && depth <= 7 {
    let margin = 80 * depth as i16;
    if static_eval - margin >= beta {
        return static_eval - margin;
    }
}
```

**Intuition**: If we're already winning by more than any reasonable swing, searching won't change the outcome.

### Late Move Reductions (LMR)

Moves searched late are likely not best; search them with reduced depth.

```rust
let reduction = if moves_searched >= 4
    && depth >= 3
    && !mv.is_tactical()
    && !in_check
    && !new_pos.is_in_check()
{
    let mut r = LMR_TABLE[depth][moves_searched];
    if !is_pv { r += 1; }
    r.min(depth - 1)
} else {
    0
};

// Search with reduction
let score = -new_pos.negamax(depth - 1 - reduction, ...);

// Re-search at full depth if reduced search exceeds alpha
if score > alpha && reduction > 0 {
    score = -new_pos.negamax(depth - 1, ...);
}
```

**LMR Table Formula:**
```
reduction = 0.75 + ln(depth) * ln(move_number) / 2.25
```

### Check Extensions

Extend search depth when in check to avoid horizon effects:

```rust
let depth = if in_check { depth + 1 } else { depth };
```

### Mate Distance Pruning

Don't search for mates beyond what we've already found:

```rust
let mating_score = MATE_SCORE - ply as i16;
if mating_score < beta {
    if mating_score <= alpha {
        return alpha;
    }
}
```

## Move Ordering

Good move ordering is critical for alpha-beta efficiency. Optimal ordering achieves O(√N) vs O(N) for random ordering.

### Ordering Priority

1. **TT Move** (score: 10,000,000)
   - Best move from previous search of this position

2. **Good Captures** (score: 8,000,000 + MVV-LVA)
   - SEE >= 0 (winning or equal exchange)
   - Ordered by Most Valuable Victim - Least Valuable Attacker

3. **Killer Moves** (score: 6,000,000 / 5,000,000)
   - Quiet moves that caused beta cutoff at this ply
   - Two killers stored per ply

4. **Counter Move** (score: 4,000,000)
   - Move that refuted opponent's previous move
   - Indexed by [previous_from][previous_to]

5. **History Heuristic** (variable score)
   - Quiet moves that caused cutoffs in past searches
   - Indexed by [color][from][to]

6. **Bad Captures** (score: -2,000,000 + MVV-LVA)
   - SEE < 0 (losing exchanges)

### MVV-LVA Scoring

```
Victim Value:  P=1, N=3, B=3, R=5, Q=9, K=10
Attacker Value: P=1, N=3, B=3, R=5, Q=9, K=10

Score = VictimValue * 10 - AttackerValue

Examples:
  PxQ = 90 - 1 = 89 (great!)
  QxP = 10 - 9 = 1  (probably bad)
```

### Selection Sort

We don't sort the entire move list upfront. Instead, we find the best remaining move each iteration:

```rust
pub fn pick_move(list: &mut MoveList, start: usize) -> Move {
    let mut best_idx = start;
    let mut best_score = list.score(start);

    for i in (start + 1)..list.len() {
        if list.score(i) > best_score {
            best_score = list.score(i);
            best_idx = i;
        }
    }

    list.swap(start, best_idx);
    list.get(start)
}
```

**Why Selection Sort?**: We often get a beta cutoff early and never look at most moves. Full sorting would waste time.

## Quiescence Search

At depth 0, we enter quiescence search to avoid horizon effects.

### Problem: Horizon Effect

Without quiescence, a queen capture on the horizon looks like a free queen, when actually our queen gets recaptured next move.

### Solution: Search Captures to Quiet Position

```rust
fn qsearch(&self, mut alpha: i16, beta: i16, ply: i32, ...) -> i16 {
    // Stand pat: option to not make any capture
    let stand_pat = self.evaluate();

    if stand_pat >= beta {
        return stand_pat;
    }
    if stand_pat > alpha {
        alpha = stand_pat;
    }

    // Generate and search captures
    let mut moves = MoveList::new();
    self.generate_captures(&mut moves);

    for mv in moves {
        // SEE pruning: skip clearly losing captures
        if !self.see_ge(mv, 0) { continue; }

        // Delta pruning: skip if can't raise alpha
        let captured_value = piece_value(captured_piece);
        if stand_pat + captured_value + 200 < alpha { continue; }

        let new_pos = self.make_move(mv);
        let score = -new_pos.qsearch(-beta, -alpha, ply + 1, ...);

        if score >= beta { return score; }
        if score > alpha { alpha = score; }
    }

    alpha
}
```

### Key Concepts

**Stand Pat**: The option to not capture anything. If our position is already good, forcing captures might make it worse.

**Delta Pruning**: If the captured piece plus a margin can't raise alpha, skip the capture.

**SEE Pruning**: Skip captures that lose material according to Static Exchange Evaluation.

## Time Management

### Time Allocation

```rust
fn calculate_time(our_time: u64, our_inc: u64, moves_to_go: u32) -> Duration {
    let moves = moves_to_go.max(1);

    // Base time: divide remaining by expected moves
    let base = our_time / moves;

    // Add most of increment
    let total = base + (our_inc * 3) / 4;

    // Keep buffer to avoid flagging
    let limit = total.min(our_time - 100);

    Duration::from_millis(limit)
}
```

### Stopping Search

Search checks for timeout periodically:

```rust
if nodes & 2047 == 0 && should_stop() {
    return 0;  // Abort search
}
```

The `& 2047` check means we only check every 2048 nodes, reducing overhead.

## Search Statistics

During search, Kai tracks:

- **Nodes**: Total positions evaluated
- **Depth**: Current search depth
- **Selective Depth**: Maximum ply reached (including extensions)
- **Time**: Elapsed search time
- **NPS**: Nodes per second
- **PV**: Principal variation (best line)
- **Score**: Evaluation in centipawns or mate distance

Example output:
```
info depth 12 seldepth 18 score cp 35 nodes 2847561 nps 1856432 time 1534 pv e2e4 e7e5 g1f3 b8c6 f1b5 a7a6 b5a4 g8f6 e1g1 f8e7
```

## Future Improvements

Potential search enhancements:

1. **Singular Extensions**: Extend search when one move is significantly better
2. **Multi-Cut Pruning**: Prune if multiple moves cause beta cutoff
3. **Internal Iterative Deepening**: Search at reduced depth when no TT move
4. **Razoring**: Prune at frontier nodes if static eval is far below alpha
5. **Lazy SMP**: Parallel search on multiple threads
6. **Aspiration Window Tuning**: Dynamic window sizing based on position

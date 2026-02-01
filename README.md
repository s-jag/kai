# Kai Chess Engine

A UCI-compatible chess engine written in Rust, implementing modern chess programming techniques for strong and efficient play.

## Features

- **Bitboard Representation**: Efficient 64-bit board representation with magic bitboards for sliding piece attacks
- **Legal Move Generation**: Fully legal move generation with perft-validated correctness
- **Alpha-Beta Search**: Negamax search with principal variation search (PVS) and various pruning techniques
- **Transposition Table**: Zobrist hashing with depth-preferred replacement policy
- **Tapered Evaluation**: Smooth interpolation between middlegame and endgame evaluation
- **UCI Protocol**: Full Universal Chess Interface support for GUI compatibility

## Building

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.70 or later

### Compilation

```bash
# Clone the repository
git clone https://github.com/s-jag/kai.git
cd kai

# Build in release mode (recommended for performance)
cargo build --release

# The binary will be at ./target/release/kai
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_perft_startpos
```

## Usage

### As a UCI Engine

Kai implements the Universal Chess Interface (UCI) protocol, making it compatible with any UCI-compliant chess GUI such as:

- [Arena](http://www.playwitharena.de/)
- [Cute Chess](https://cutechess.com/)
- [Banksia GUI](https://banksiagui.com/)
- [Lucas Chess](https://lucaschess.pythonanywhere.com/)

Simply add the `kai` binary as a new engine in your preferred GUI.

### Command Line

```bash
# Start the engine
./target/release/kai

# The engine will wait for UCI commands
uci
isready
position startpos moves e2e4 e7e5
go depth 10
quit
```

### UCI Commands

| Command | Description |
|---------|-------------|
| `uci` | Initialize UCI mode, display engine info |
| `isready` | Check if engine is ready |
| `ucinewgame` | Reset engine state for a new game |
| `position startpos` | Set starting position |
| `position startpos moves e2e4 e7e5` | Set position with moves |
| `position fen <fen>` | Set position from FEN string |
| `go depth <n>` | Search to depth n |
| `go movetime <ms>` | Search for specified milliseconds |
| `go wtime <ms> btime <ms>` | Search with time controls |
| `go infinite` | Search until stopped |
| `stop` | Stop searching |
| `quit` | Exit the engine |
| `setoption name Hash value <mb>` | Set hash table size (1-4096 MB) |

### Debug Commands

| Command | Description |
|---------|-------------|
| `d` | Display the current board position |
| `perft <depth>` | Run perft test with divide output |
| `eval` | Show static evaluation of current position |

## Architecture

### Project Structure

```
kai/
├── Cargo.toml              # Project configuration
├── README.md               # This file
├── docs/
│   ├── ARCHITECTURE.md     # Detailed architecture documentation
│   ├── SEARCH.md           # Search algorithm details
│   └── EVALUATION.md       # Evaluation function details
└── src/
    ├── main.rs             # Entry point
    ├── lib.rs              # Library exports
    ├── types.rs            # Core types (Square, Piece, Color)
    ├── bitboard.rs         # Bitboard operations
    ├── magic.rs            # Magic bitboard tables
    ├── position.rs         # Board state and FEN parsing
    ├── moves.rs            # Move encoding
    ├── movegen.rs          # Move generation
    ├── make_move.rs        # Make/unmake move
    ├── zobrist.rs          # Zobrist hashing
    ├── tt.rs               # Transposition table
    ├── eval.rs             # Position evaluation
    ├── search.rs           # Main search algorithm
    ├── qsearch.rs          # Quiescence search
    ├── see.rs              # Static exchange evaluation
    ├── ordering.rs         # Move ordering
    ├── uci.rs              # UCI protocol
    └── perft.rs            # Perft testing
```

### Key Components

#### Board Representation
- **Bitboards**: 12 bitboards (one per piece type per color) for efficient move generation
- **Mailbox**: 64-element array for O(1) piece lookup
- **Magic Bitboards**: Precomputed attack tables for sliding pieces (bishops, rooks, queens)

#### Search
- **Iterative Deepening**: Progressively deeper searches with aspiration windows
- **Principal Variation Search (PVS)**: Null-window search for non-PV nodes
- **Pruning Techniques**:
  - Null Move Pruning
  - Reverse Futility Pruning (Static Null Move Pruning)
  - Late Move Reductions (LMR)
- **Extensions**: Check extensions

#### Evaluation
- **Material**: Standard piece values
- **Piece-Square Tables**: PeSTO values for positional scoring
- **Pawn Structure**: Doubled, isolated, and passed pawn evaluation
- **Tapered Evaluation**: Smooth middlegame to endgame transition

#### Move Ordering
1. Transposition table move
2. Good captures (positive SEE) ordered by MVV-LVA
3. Killer moves (2 per ply)
4. Counter moves
5. History heuristic for quiet moves
6. Bad captures (negative SEE)

## Performance

### Perft Results

| Position | Depth | Nodes | Verified |
|----------|-------|-------|----------|
| Starting | 5 | 4,865,609 | ✓ |
| Kiwipete | 4 | 4,085,603 | ✓ |
| Position 3 | 5 | 674,624 | ✓ |
| Position 4 | 4 | 422,333 | ✓ |
| Position 5 | 4 | 2,103,487 | ✓ |
| Position 6 | 4 | 3,894,594 | ✓ |

### Search Speed

Typical performance on modern hardware:
- ~1-3 million nodes per second in search
- ~10-50 million nodes per second in perft

## Configuration

### Hash Table Size

The default hash table size is 64 MB. Adjust based on your system's available memory:

```
setoption name Hash value 256
```

Recommended sizes:
- 64-128 MB for casual play
- 256-512 MB for analysis
- 1024+ MB for long time control games

## Technical Details

### Move Encoding

Moves are encoded in 16 bits:
- Bits 0-5: Source square (0-63)
- Bits 6-11: Destination square (0-63)
- Bits 12-15: Move flags (quiet, capture, promotion, castling, en passant)

### Zobrist Hashing

Position hashing uses Zobrist keys:
- 768 piece-square keys (2 colors x 6 piece types x 64 squares)
- 16 castling right keys
- 8 en passant file keys
- 1 side-to-move key

### Transposition Table Entry

Each TT entry is 16 bytes:
- 4 bytes: Hash key verification
- 2 bytes: Best move
- 2 bytes: Score
- 1 byte: Depth
- 1 byte: Bound type
- 1 byte: Age
- 5 bytes: Padding (alignment)

## Contributing

Contributions are welcome! Areas of interest:

- **Search improvements**: Singular extensions, multi-cut pruning
- **Evaluation**: Mobility, king safety, piece activity
- **NNUE**: Neural network evaluation
- **Multi-threading**: Lazy SMP parallel search
- **Endgame tablebases**: Syzygy support

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Chess Programming Wiki](https://www.chessprogramming.org/) - Invaluable resource for chess programming
- [Stockfish](https://stockfishchess.org/) - Inspiration for many techniques
- [PeSTO](https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function) - Piece-square table values
- [Rustic Chess](https://rustic-chess.org/) - Rust chess engine reference

## Author

Sahith Jagarlamudi

---

*Kai is named after the concept of continuous improvement in Japanese philosophy (改善, kaizen).*

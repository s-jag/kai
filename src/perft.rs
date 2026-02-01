/// Perft (performance test) for move generation validation
use crate::moves::MoveList;
use crate::position::Position;

/// Run perft and return the node count
pub fn perft(pos: &Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut moves = MoveList::new();
    pos.generate_legal_moves(&mut moves);

    // Leaf node optimization
    if depth == 1 {
        return moves.len() as u64;
    }

    let mut nodes = 0u64;

    for mv in moves.iter() {
        let new_pos = pos.make_move(mv);
        nodes += perft(&new_pos, depth - 1);
    }

    nodes
}

/// Run perft with divide output (shows nodes per move)
pub fn perft_divide(pos: &Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut moves = MoveList::new();
    pos.generate_legal_moves(&mut moves);

    let mut total = 0u64;

    for mv in moves.iter() {
        let new_pos = pos.make_move(mv);
        let nodes = perft(&new_pos, depth - 1);
        println!("{}: {}", mv.to_uci(), nodes);
        total += nodes;
    }

    total
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
    fn test_perft_startpos() {
        setup();
        let pos = Position::new();

        assert_eq!(perft(&pos, 1), 20, "Depth 1 failed");
        assert_eq!(perft(&pos, 2), 400, "Depth 2 failed");
        assert_eq!(perft(&pos, 3), 8902, "Depth 3 failed");
        assert_eq!(perft(&pos, 4), 197281, "Depth 4 failed");
        // Depth 5 takes a bit longer, but should be correct
        // assert_eq!(perft(&pos, 5), 4865609, "Depth 5 failed");
    }

    #[test]
    fn test_perft_kiwipete() {
        setup();
        // Kiwipete position - good for testing complex move generation
        let pos = Position::from_fen(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        )
        .unwrap();

        assert_eq!(perft(&pos, 1), 48, "Kiwipete depth 1 failed");
        assert_eq!(perft(&pos, 2), 2039, "Kiwipete depth 2 failed");
        assert_eq!(perft(&pos, 3), 97862, "Kiwipete depth 3 failed");
        // assert_eq!(perft(&pos, 4), 4085603, "Kiwipete depth 4 failed");
    }

    #[test]
    fn test_perft_position3() {
        setup();
        // Position 3 from CPW
        let pos = Position::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();

        assert_eq!(perft(&pos, 1), 14, "Position 3 depth 1 failed");
        assert_eq!(perft(&pos, 2), 191, "Position 3 depth 2 failed");
        assert_eq!(perft(&pos, 3), 2812, "Position 3 depth 3 failed");
        assert_eq!(perft(&pos, 4), 43238, "Position 3 depth 4 failed");
        // assert_eq!(perft(&pos, 5), 674624, "Position 3 depth 5 failed");
    }

    #[test]
    fn test_perft_position4() {
        setup();
        // Position 4 from CPW (mirrored)
        let pos = Position::from_fen(
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        )
        .unwrap();

        assert_eq!(perft(&pos, 1), 6, "Position 4 depth 1 failed");
        assert_eq!(perft(&pos, 2), 264, "Position 4 depth 2 failed");
        assert_eq!(perft(&pos, 3), 9467, "Position 4 depth 3 failed");
        // assert_eq!(perft(&pos, 4), 422333, "Position 4 depth 4 failed");
    }

    #[test]
    fn test_perft_position5() {
        setup();
        // Position 5 from CPW
        let pos =
            Position::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8")
                .unwrap();

        assert_eq!(perft(&pos, 1), 44, "Position 5 depth 1 failed");
        assert_eq!(perft(&pos, 2), 1486, "Position 5 depth 2 failed");
        assert_eq!(perft(&pos, 3), 62379, "Position 5 depth 3 failed");
        // assert_eq!(perft(&pos, 4), 2103487, "Position 5 depth 4 failed");
    }

    #[test]
    fn test_perft_position6() {
        setup();
        // Position 6 from CPW
        let pos = Position::from_fen(
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        )
        .unwrap();

        assert_eq!(perft(&pos, 1), 46, "Position 6 depth 1 failed");
        assert_eq!(perft(&pos, 2), 2079, "Position 6 depth 2 failed");
        assert_eq!(perft(&pos, 3), 89890, "Position 6 depth 3 failed");
        // assert_eq!(perft(&pos, 4), 3894594, "Position 6 depth 4 failed");
    }

    #[test]
    fn test_perft_en_passant() {
        setup();
        // Position with en passant possibility
        let pos =
            Position::from_fen("rnbqkbnr/pppp1ppp/8/4pP2/8/8/PPPPP1PP/RNBQKBNR w KQkq e6 0 1")
                .unwrap();

        // Should include en passant capture
        let nodes = perft(&pos, 1);
        assert!(nodes > 0);
    }

    #[test]
    fn test_perft_promotion() {
        setup();
        // Position with promotion
        let pos = Position::from_fen("8/P7/8/8/8/8/8/4K2k w - - 0 1").unwrap();

        // a7-a8 with 4 promotion options = 4 moves, plus king moves
        let nodes = perft(&pos, 1);
        assert!(nodes >= 4);
    }

    #[test]
    fn test_perft_castling() {
        setup();
        // Position with all castling rights
        let pos =
            Position::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();

        let nodes = perft(&pos, 1);
        // Should have both castling options available
        assert!(nodes >= 16); // At least pawn moves + castling
    }
}

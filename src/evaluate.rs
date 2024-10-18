use shakmaty::{Board, Chess, Position};

pub fn evaluate(board: &Chess) -> f32 {
    let mut evaluation: f32 = 0.0;

    let bitboard: &Board = board.board();

    evaluation += 1.0 * (bitboard.pawns() & bitboard.white()).count() as f32; // White pawns
    evaluation -= 1.0 * (bitboard.pawns() & bitboard.black()).count() as f32; // Black pawns

    evaluation += 3.2 * (bitboard.bishops() & bitboard.white()).count() as f32; // White bishops
    evaluation -= 3.2 * (bitboard.bishops() & bitboard.black()).count() as f32; // Black bishops

    evaluation += 3.0 * (bitboard.knights() & bitboard.white()).count() as f32; // White knights
    evaluation -= 3.0 * (bitboard.knights() & bitboard.black()).count() as f32; // Black bishops

    evaluation += 5.0 * (bitboard.rooks() & bitboard.white()).count() as f32; // White rooks
    evaluation -= 5.0 * (bitboard.rooks() & bitboard.black()).count() as f32; // Black rooks

    evaluation += 9.0 * (bitboard.queens() & bitboard.white()).count() as f32; // White queen
    evaluation -= 9.0 * (bitboard.queens() & bitboard.black()).count() as f32; // Black queen

    return evaluation;
}

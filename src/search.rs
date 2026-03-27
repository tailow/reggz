use crate::engine::TRANSPOSITION_TABLE_LENGTH;
use crate::evaluate::evaluate;
use shakmaty::zobrist::Zobrist64;
use shakmaty::{CastlingMode, Chess, Color, EnPassantMode, Move, MoveList, Position, Role};
use std::f32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

// Node types for transposition table entries, indicating the accuracy of stored scores
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Exact,      // PV-node
    Upperbound, // All-node
    Lowerbound, // Cut-node
}

// Transposition table node storing search results for a given position
#[derive(Debug, Clone)]
pub struct Node {
    pub hash: Zobrist64,
    pub score: i16,
    pub best_move: Option<Move>,
    pub depth: i16,
    pub node_type: NodeType,
}

pub struct Searcher {
    pub nodes: u64,
    pub searching: Arc<AtomicBool>,
    pub max_depth: Option<i16>,
    pub debug: Arc<AtomicBool>,
    pub best_root_move: Option<Move>,
}

pub const MATE: i16 = 31000; // Base value for checkmate (offset by ply to show mate distance)
pub const MATE_MAX_PLIES: i16 = 128; // Maximum plies to consider for mate scoring

#[allow(dead_code)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub ponder_move: Option<Move>,
    pub score: Option<i16>,
}

impl Searcher {
    pub fn search(
        &mut self,
        board: Chess,
        position_history: &mut Vec<Zobrist64>,
        transposition_table: &mut Arc<Mutex<Vec<Option<Node>>>>,
    ) {
        let mut tt = transposition_table.lock().unwrap();

        let result = self.search_impl(board, position_history, &mut tt);

        if let Some(best_move) = result.best_move {
            if let Some(ponder_move) = result.ponder_move {
                println!(
                    "bestmove {} ponder {}",
                    best_move.to_uci(CastlingMode::Standard),
                    ponder_move.to_uci(CastlingMode::Standard)
                );
            } else {
                println!("bestmove {}", best_move.to_uci(CastlingMode::Standard));
            }
        }

        self.searching.store(false, Ordering::Relaxed);
    }

    fn search_impl(
        &mut self,
        board: Chess,
        position_history: &mut Vec<Zobrist64>,
        transposition_table: &mut [Option<Node>],
    ) -> SearchResult {
        let mut score: Option<i16> = None;
        let mut previous_score: Option<i16> = None;

        let mut ponder_move: Option<Move> = None;

        let start_time = SystemTime::now();

        let mut principal_variation: Vec<Move>;

        let hash = board.zobrist_hash::<Zobrist64>(EnPassantMode::Legal);

        let mut max_depth: i16 = i16::MAX;

        if let Some(custom_max_depth) = self.max_depth {
            max_depth = custom_max_depth;
        }

        // Iterative deepening
        for depth in 1..max_depth {
            if !self.searching.load(Ordering::Relaxed) {
                break;
            }

            // Aspiration window: narrow search window around expected score
            let mut lower_window = i16::MIN + 1;
            let mut upper_window = i16::MAX - 1;

            if let Some(previous_score) = previous_score {
                lower_window = previous_score - 50;
                upper_window = previous_score + 50;
            }

            // Aspiration loop: retry with wider windows if search fails
            loop {
                let mut alpha = lower_window;
                let mut beta = upper_window;

                score = self.negamax(
                    &board,
                    depth,
                    0,
                    &mut alpha,
                    &mut beta,
                    if board.turn() == Color::White { 1 } else { -1 },
                    position_history,
                    hash,
                    transposition_table,
                );

                if let Some(score) = score {
                    if score <= lower_window {
                        // Failed low: widen window downward
                        lower_window = score - 100;
                    } else if score >= upper_window {
                        // Failed high: widen window upward
                        upper_window = score + 100;
                    } else {
                        // Score within window: success
                        break;
                    }
                } else {
                    break;
                }
            }

            if let Some(s) = score {
                previous_score = Some(s);

                principal_variation =
                    self.get_principal_variation(&mut board.clone(), depth, transposition_table);

                if principal_variation.len() > 1 {
                    ponder_move = Some(principal_variation[1]);
                }

                if self.debug.load(Ordering::Relaxed) {
                    self.print_info(s, start_time, depth, &principal_variation);
                }
            }
        }

        SearchResult {
            best_move: self.best_root_move,
            ponder_move,
            score,
        }
    }

    // Output search information in UCI format
    fn print_info(
        &mut self,
        score: i16,
        start_time: SystemTime,
        depth: i16,
        principal_variation: &[Move],
    ) {
        let time_ms = start_time.elapsed().unwrap().as_millis();
        let nodes_per_second: u64 = self.nodes / (time_ms + 1) as u64 * 1000;

        let pv_string: String = principal_variation
            .iter()
            .map(|m| m.to_uci(CastlingMode::Standard).to_string())
            .collect::<Vec<String>>()
            .join(" ");

        // Format score as mate in N moves or centipawns
        let score_string = if score > MATE - MATE_MAX_PLIES {
            let mate_in_plies = MATE - score;

            format!("mate {}", ((mate_in_plies as f64 / 2.0).ceil() as i8))
        } else if score < -MATE + MATE_MAX_PLIES {
            let mate_in_plies = -MATE - score;

            format!("mate {}", ((mate_in_plies as f64 / 2.0).ceil() as i8))
        } else {
            format!("cp {}", score)
        };

        println!("info depth {depth} score {score_string} time {time_ms} nodes {0} nps {nodes_per_second} pv {pv_string}", self.nodes);

        self.nodes = 0;
    }

    // Quiescence search: extends search at leaf nodes to avoid horizon effect
    // Only searches captures to find quiet positions for accurate evaluation
    #[allow(clippy::too_many_arguments)]
    fn quiesce(
        &mut self,
        board: &Chess,
        alpha: &mut i16,
        beta: &mut i16,
        color: i16,
        ply: i16,
        hash: Zobrist64,
        position_history: &mut Vec<Zobrist64>,
        transposition_table: &[Option<Node>],
    ) -> Option<i16> {
        if board.is_insufficient_material() {
            return Some(0);
        }

        // Checkmate or stalemate
        if board.legal_moves().is_empty() {
            if !board.checkers().is_empty() {
                return Some(-MATE + ply);
            }

            return Some(0);
        }
        // 50-move rule
        else if board.halfmoves() >= 100 {
            return Some(0);
        }
        // Repetition
        if ply > 0 {
            let mut repetitions: u8 = 0;

            for position in position_history.iter().rev().step_by(2) {
                if *position == hash {
                    repetitions += 1;

                    if repetitions >= 2 {
                        return Some(0);
                    }
                }
            }
        }

        let evaluation = color * evaluate(board);

        let mut best_score: i16 = evaluation;

        // Beta cutoff: stand pat if current eval is already good enough
        if best_score >= *beta {
            return Some(best_score);
        }

        if best_score > *alpha {
            *alpha = best_score;
        }

        let mut capture_moves = board.capture_moves();

        self.sort_legal_moves(&mut capture_moves, board, hash, transposition_table);

        for capture_move in capture_moves {
            self.nodes += 1;

            let mut board_clone = board.clone();

            let child_hash;

            // TODO: Unmake move
            if let Some(new_child_hash) =
                board_clone.update_zobrist_hash(hash, capture_move, EnPassantMode::Legal)
            {
                child_hash = new_child_hash;

                board_clone.play_unchecked(capture_move);
            } else {
                board_clone.play_unchecked(capture_move);

                child_hash = board_clone.zobrist_hash(EnPassantMode::Legal);
            }

            position_history.push(child_hash);

            let move_score = -self.quiesce(
                &board_clone,
                &mut -(*beta),
                &mut alpha.wrapping_neg(),
                -color,
                ply + 1,
                child_hash,
                position_history,
                transposition_table,
            )?;

            position_history.pop();

            if move_score > best_score {
                best_score = move_score;

                if move_score > *alpha {
                    *alpha = move_score;
                }
            }

            if move_score >= *beta {
                break;
            }

            if !self.searching.load(Ordering::Relaxed) {
                return None;
            }
        }

        Some(best_score)
    }

    // Negamax algorithm with alpha-beta pruning
    #[allow(clippy::too_many_arguments)]
    fn negamax(
        &mut self,
        board: &Chess,
        depth: i16,
        ply: i16,
        alpha: &mut i16,
        beta: &mut i16,
        color: i16,
        position_history: &mut Vec<Zobrist64>,
        hash: Zobrist64,
        transposition_table: &mut [Option<Node>],
    ) -> Option<i16> {
        let mut best_score = i16::MIN + 1;

        if board.is_insufficient_material() {
            return Some(0);
        }

        let mut legal_moves: MoveList = board.legal_moves();

        // Checkmate or stalemate
        if legal_moves.is_empty() {
            if !board.checkers().is_empty() {
                return Some(-MATE + ply);
            }

            return Some(0);
        }
        // 50-move rule
        else if board.halfmoves() >= 100 {
            return Some(0);
        }
        // Repetition
        if ply > 0 {
            let mut repetitions: u8 = 0;

            for position in position_history.iter().rev().step_by(2) {
                if *position == hash {
                    repetitions += 1;

                    if repetitions >= 2 {
                        return Some(0);
                    }
                }
            }
        }

        // Leaf node
        if depth <= 0 {
            return self.quiesce(
                board,
                alpha,
                beta,
                color,
                ply,
                hash,
                position_history,
                transposition_table,
            );
        }

        let transposition_table_index: usize = hash.0 as usize % TRANSPOSITION_TABLE_LENGTH;

        // Transposition table hit: reuse previously computed results
        if let Some(ref tt_node) = transposition_table[transposition_table_index] {
            if tt_node.hash == hash && tt_node.depth >= depth {
                let node = tt_node.clone();

                if node.node_type == NodeType::Exact {
                    return Some(node.score);
                } else if node.node_type == NodeType::Lowerbound {
                    *alpha = i16::max(*alpha, node.score);
                } else if node.node_type == NodeType::Upperbound {
                    *beta = i16::min(*beta, node.score);
                }
                if alpha >= beta {
                    return Some(node.score);
                }
            }
        }

        // Nullmove pruning
        /*
        let only_pawns = (board.turn().is_black()
            && (board.board().black() & (board.board().pawns() | board.board().kings())
                == board.board().black()))
            || (board.turn().is_white()
                && (board.board().white() & (board.board().pawns() | board.board().kings())
                    == board.board().white()));

        if !board.is_check() && !only_pawns && depth >= 3 {
            let reduction: i16 = depth / 3 + 2;

            let board_clone = board.clone().swap_turn().unwrap();

            let child_hash = board_clone.zobrist_hash(EnPassantMode::Legal);

            let move_score = -self.negamax(
                &board_clone,
                depth - reduction,
                ply + 1,
                &mut -(*beta),
                &mut -(*beta + 1),
                -color,
                position_history,
                child_hash,
                transposition_table,
            )?;

            if move_score >= *beta {
                return Some(move_score);
            }
        }
        */

        let mut node: Node = Node {
            best_move: None,
            depth,
            hash,
            node_type: NodeType::Upperbound,
            score: best_score,
        };

        self.sort_legal_moves(&mut legal_moves, board, hash, transposition_table);

        for (i, legal_move) in legal_moves.iter().enumerate() {
            self.nodes += 1;

            let mut board_clone = board.clone();

            let child_hash;

            // TODO: Unmake move
            if let Some(new_child_hash) =
                board_clone.update_zobrist_hash(hash, *legal_move, EnPassantMode::Legal)
            {
                child_hash = new_child_hash;

                board_clone.play_unchecked(*legal_move);
            } else {
                board_clone.play_unchecked(*legal_move);

                child_hash = board_clone.zobrist_hash(EnPassantMode::Legal);
            }

            position_history.push(child_hash);

            let mut extension: i16 = 0;
            let mut reduction: i16 = 0;

            // Check extension
            if board_clone.is_check() {
                extension = 1;
            }
            // Late move reduction: reduce depth for later moves (LMP)
            else if depth > 2 && i > 2 {
                reduction = (0.99 + f32::ln(depth.into()) * f32::ln((i) as f32) / f32::consts::PI)
                    .floor() as i16;
            }

            let mut move_score = -self.negamax(
                &board_clone,
                depth - 1 + extension - reduction,
                ply + 1,
                &mut -(*beta),
                &mut alpha.wrapping_neg(),
                -color,
                position_history,
                child_hash,
                transposition_table,
            )?;

            // Don't reduce depth if reduced search fails high
            if reduction > 0 && move_score > *alpha {
                move_score = -self.negamax(
                    &board_clone,
                    depth - 1 + extension,
                    ply + 1,
                    &mut -(*beta),
                    &mut alpha.wrapping_neg(),
                    -color,
                    position_history,
                    child_hash,
                    transposition_table,
                )?;
            }

            position_history.pop();

            if move_score > best_score {
                best_score = move_score;

                node.best_move = Some(*legal_move);

                // Best root move
                if ply == 0 {
                    self.best_root_move = Some(*legal_move);
                }

                // Found the best guaranteed move
                if move_score > *alpha {
                    *alpha = move_score;

                    node.node_type = NodeType::Exact;
                }
            }

            // Beta cutoff / fail high: found a move that is too good, causing the opponent to avoid
            // this node
            if move_score >= *beta {
                node.node_type = NodeType::Lowerbound;

                break;
            }

            if !self.searching.load(Ordering::Relaxed) && ply > 0 {
                return None;
            }
        }

        node.score = best_score;

        // Store node in the transposition table
        if self.searching.load(Ordering::Relaxed) {
            if let Some(ref tt_node) = transposition_table[transposition_table_index] {
                if tt_node.depth <= depth {
                    transposition_table[transposition_table_index] = Some(node.clone());
                }
            } else {
                transposition_table[transposition_table_index] = Some(node.clone());
            }
        }

        Some(best_score)
    }

    // Move ordering
    fn sort_legal_moves(
        &self,
        legal_moves: &mut MoveList,
        board: &Chess,
        hash: Zobrist64,
        transposition_table: &[Option<Node>],
    ) {
        if legal_moves.is_empty() {
            return;
        }

        // Move best move from transposition table to the front
        if let Some(ref pv_node) = transposition_table[hash.0 as usize % TRANSPOSITION_TABLE_LENGTH]
        {
            if let Some(best_move) = pv_node.best_move {
                if board.is_legal(best_move) {
                    if let Some(pos) = legal_moves.iter().position(|m| *m == best_move) {
                        legal_moves.swap(0, pos);
                    }
                }
            }
        }

        // Score each move for sorting using MVV-LVA heuristic
        legal_moves[1..].sort_by_cached_key(|m| {
            if m.is_promotion() {
                return 0i16; // Promotions first
            }
            if m.is_capture() {
                // MVV-LVA: Most Valuable Victim - Least Valuable Attacker
                // Prioritize capturing high-value pieces with low-value pieces
                let victim = match m.capture().unwrap() {
                    Role::Pawn => 100,
                    Role::Knight => 300,
                    Role::Bishop => 350,
                    Role::Rook => 500,
                    Role::Queen => 900,
                    Role::King => return 0, // shouldn't happen
                };
                let attacker = match m.role() {
                    Role::Pawn => 100,
                    Role::Knight => 300,
                    Role::Bishop => 350,
                    Role::Rook => 500,
                    Role::Queen => 900,
                    Role::King => 2000,
                };
                return -(victim - attacker + 1000); // negative = higher priority; +1000 ensures captures beat quiets
            }
            1000i16 // quiet moves last
        });
    }

    // Reconstruct the principal variation (best line) from transposition table
    fn get_principal_variation(
        &self,
        board: &mut Chess,
        depth: i16,
        transposition_table: &[Option<Node>],
    ) -> Vec<Move> {
        let mut pv: Vec<Move> = Vec::new();

        let mut hash: Zobrist64;

        for _ in 0..depth {
            hash = board.zobrist_hash::<Zobrist64>(EnPassantMode::Legal);

            if let Some(ref pv_node) =
                transposition_table[hash.0 as usize % TRANSPOSITION_TABLE_LENGTH]
            {
                if let Some(best_move) = pv_node.best_move {
                    if board.is_legal(best_move) {
                        pv.push(best_move);

                        board.play_unchecked(best_move);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        pv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::{fen::Fen, Chess};

    // Helper to parse FEN string into Chess position
    fn parse_fen(fen_str: &str) -> Chess {
        let fen: Fen = Fen::from_ascii(fen_str.as_bytes()).unwrap();
        fen.into_position(shakmaty::CastlingMode::Standard).unwrap()
    }

    // Helper to run full search with iterative deepening and aspiration windows
    fn run_search(board: Chess, max_depth: i16) -> SearchResult {
        let mut searcher = Searcher {
            nodes: 0,
            searching: Arc::new(AtomicBool::new(true)),
            max_depth: Some(max_depth),
            debug: Arc::new(AtomicBool::new(false)),
            best_root_move: None,
        };

        let mut position_history = Vec::new();
        let mut tt = vec![None; TRANSPOSITION_TABLE_LENGTH];

        searcher.search_impl(board, &mut position_history, &mut tt)
    }

    #[test]
    fn test_mate_in_1_white_score() {
        // White queen mates on h7 - should return a mate score
        let fen = "r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5Q2/PPPP1PPP/RNB1K1NR w KQkq - 4 3";
        let board = parse_fen(fen);

        let result = run_search(board, 5);

        // Mate in 1 for white should be a positive mate score
        let score = result.score.unwrap();
        assert!(
            score > MATE - MATE_MAX_PLIES,
            "Expected mate score, got {}",
            score
        );
    }

    #[test]
    fn test_mate_in_1_black_score() {
        // Black to play and mate in 1 with ...Qh1#
        let fen = "4q1k1/5ppp/8/8/8/8/5PPP/6K1 b - - 0 1";
        let board = parse_fen(fen);

        let result = run_search(board, 5);

        // Mate in 1 for black should be a negative mate score
        let score = result.score.unwrap();
        assert!(
            score > MATE - MATE_MAX_PLIES,
            "Expected mate score for black, got {}",
            score
        );
    }

    #[test]
    fn test_material_advantage_score() {
        // Black is up a rook
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQK1NR w KQkq - 0 1";
        let board = parse_fen(fen);

        let result = run_search(board, 3);

        let score = result.score.unwrap();
        // Black is up a rook (525 points), score should be significantly positive
        assert!(
            score < -300,
            "Black should have significant advantage, got {}",
            score
        );
    }

    #[test]
    fn test_symmetrical_position_score() {
        // Starting position is symmetrical, should be near zero
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let board = parse_fen(fen);

        let result = run_search(board, 3);

        let score = result.score.unwrap();
        // Should be close to 0 (within reasonable margin for positional differences)
        assert!(
            score.abs() < 100,
            "Symmetrical position should be near 0, got {}",
            score
        );
    }

    #[test]
    fn test_fifty_move_rule_draw() {
        // Test 50-move rule detection
        let fen = "6k1/5ppp/8/8/8/8/5PPP/5RK1 w - - 100 1";
        let board = parse_fen(fen);

        assert_eq!(board.halfmoves(), 100);

        let result = run_search(board, 3);

        // Should return 0 for draw by 50-move rule
        assert_eq!(result.score, Some(0));
    }

    #[test]
    fn test_insufficient_material_draw() {
        // King vs King - insufficient material
        let fen = "6k1/8/8/8/8/8/8/4K3 w - - 0 1";
        let board = parse_fen(fen);

        assert!(board.is_insufficient_material());

        let result = run_search(board, 3);

        assert_eq!(result.score, Some(0));
    }
}

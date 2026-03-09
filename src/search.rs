use crate::engine::TRANSPOSITION_TABLE_LENGTH;
use crate::evaluate;
use shakmaty::zobrist::Zobrist64;
use shakmaty::{CastlingMode, Chess, Color, EnPassantMode, Move, MoveList, Position};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Exact,
    Upperbound,
    Lowerbound,
}

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
    pub _pondering: Arc<AtomicBool>,
    pub max_depth: Option<i16>,
    pub debug: Arc<AtomicBool>,
}

pub const MATE: i16 = 31000;
pub const MATE_MAX_PLIES: i16 = 128;

impl Searcher {
    pub fn search(
        &mut self,
        board: Chess,
        position_history: &mut Vec<Zobrist64>,
        transposition_table: &mut Arc<Mutex<Vec<Option<Node>>>>,
    ) {
        let mut score: Option<i16>;
        let mut previous_score: Option<i16> = None;

        let start_time = SystemTime::now();

        let mut transposition_table = transposition_table.lock().unwrap();

        let mut principal_variation: Vec<Move>;

        let hash = board.zobrist_hash::<Zobrist64>(EnPassantMode::Legal);

        let mut previous_best_move: Option<Move> = None;

        let mut max_depth: i16 = i16::MAX;

        if let Some(custom_max_depth) = self.max_depth {
            max_depth = custom_max_depth;
        }

        for depth in 1..max_depth {
            if !self.searching.load(Ordering::Relaxed) {
                break;
            }

            let mut lower_window = i16::MIN + 1;
            let mut upper_window = i16::MAX - 1;

            if let Some(previous_score) = previous_score {
                lower_window = previous_score - 250;
                upper_window = previous_score + 250;
            }

            '_aspiration: loop {
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
                    &mut transposition_table,
                );

                if let Some(score) = score {
                    if score <= lower_window {
                        lower_window = i16::MIN + 1;
                    } else if score >= upper_window {
                        upper_window = i16::MAX - 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Maybe don't discard ?
            if let Some(score) = score {
                previous_score = Some(score);

                if let Some(ref pv_node) =
                    transposition_table[hash.0 as usize % TRANSPOSITION_TABLE_LENGTH]
                {
                    if let Some(best_move) = pv_node.best_move {
                        if board.is_legal(best_move) {
                            previous_best_move = Some(best_move);
                        }
                    }
                }

                principal_variation =
                    self.get_principal_variation(&mut board.clone(), depth, &transposition_table);

                if self.debug.load(Ordering::Relaxed) {
                    self.print_info(score, start_time, depth, &principal_variation);
                }
            }
        }

        // TODO: Use proper PV for getting best move
        if let Some(best_move) = previous_best_move {
            if board.is_legal(best_move) {
                println!("bestmove {}", best_move.to_uci(CastlingMode::Standard));
            }
        }

        self.searching.store(false, Ordering::Relaxed);
    }

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

    fn negamax(
        &mut self,
        board: &Chess,
        depth: i16,
        ply: u16,
        alpha: &mut i16,
        beta: &mut i16,
        color: i16,
        position_history: &mut Vec<Zobrist64>,
        hash: Zobrist64,
        transposition_table: &mut [Option<Node>],
    ) -> Option<i16> {
        let mut score = i16::MIN + 1;

        if board.is_insufficient_material() {
            return Some(0);
        }

        let mut legal_moves: MoveList = board.legal_moves();

        // Checkmate or stalemate
        if legal_moves.is_empty() {
            if !board.checkers().is_empty() {
                return Some(-MATE);
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

        let transposition_table_index: usize = hash.0 as usize % TRANSPOSITION_TABLE_LENGTH;

        // Transposition table hit
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

        if depth <= 0 {
            return Some(color * evaluate::evaluate(board));
        }

        // Nullmove pruning
        /*
        if !board.is_check()
            && (board.board().black() | board.board().white()
                != board.board().pawns() | board.board().kings())
        {
            let reduction: i16 = 4;

            let board_clone = board.clone().swap_turn().unwrap();

            let child_hash = board_clone.zobrist_hash(EnPassantMode::Legal);

            let child_score = -self.negamax(
                board,
                depth - reduction,
                ply + 1,
                &mut -(*beta),
                &mut -(*beta - 1),
                -color,
                position_history,
                child_hash,
                transposition_table,
            )?;

            if child_score >= *beta {
                return Some(child_score);
            }
        }
        */

        let mut node: Node = Node {
            best_move: None,
            depth,
            hash,
            node_type: NodeType::Upperbound,
            score,
        };

        self.sort_legal_moves(&mut legal_moves, board, hash, transposition_table);

        for legal_move in legal_moves {
            self.nodes += 1;

            let mut board_clone = board.clone();

            let child_hash;

            // TODO: Unmake move
            if let Some(new_child_hash) =
                board_clone.update_zobrist_hash(hash, legal_move, EnPassantMode::Legal)
            {
                child_hash = new_child_hash;

                board_clone.play_unchecked(legal_move);
            } else {
                board_clone.play_unchecked(legal_move);

                child_hash = board_clone.zobrist_hash(EnPassantMode::Legal);
            }

            position_history.push(child_hash);

            let child_score = self.negamax(
                &board_clone,
                depth - 1,
                ply + 1,
                &mut -(*beta),
                &mut -(*alpha),
                -color,
                position_history,
                child_hash,
                transposition_table,
            )?;

            position_history.pop();

            if -child_score > score {
                score = -child_score;
                node.best_move = Some(legal_move);

                if score > *alpha {
                    *alpha = score;

                    node.node_type = NodeType::Exact;
                }

                // If move leads to mate
                if score > MATE - MATE_MAX_PLIES {
                    score -= 1;
                } else if score < -MATE + MATE_MAX_PLIES {
                    score += 1;
                }
            }

            if score >= *beta {
                node.node_type = NodeType::Lowerbound;

                break;
            }

            if !self.searching.load(Ordering::Relaxed) {
                return None;
            }
        }

        node.score = score;

        // Store node in the transposition table
        if self.searching.load(Ordering::Relaxed) {
            transposition_table[transposition_table_index] = Some(node.clone());
        }

        Some(score)
    }

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

        // Move best move to the front
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

        if legal_moves.len() < 3 {
            return;
        }

        // Selection sort
        for move_index in 1..legal_moves.len() - 1 {
            for inner_move_index in move_index + 1..legal_moves.len() {
                let inner_move: &Move = &legal_moves[inner_move_index];

                if inner_move.is_promotion() || inner_move.is_capture() {
                    legal_moves.swap(inner_move_index, move_index);

                    break;
                }
            }
        }
    }

    // Should probably switch to a different method
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

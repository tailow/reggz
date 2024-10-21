use crate::engine::TRANSPOSITION_TABLE_LENGTH;
use crate::evaluate;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{CastlingMode, Chess, Color, EnPassantMode, Move, MoveList, Position};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    EXACT,
    UPPERBOUND,
    LOWERBOUND,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub hash: u64,
    pub score: i16,
    pub best_move: Option<Move>,
    pub depth: u8,
    pub node_type: NodeType,
    pub mate_in_plies: Option<i8>,
    pub terminal: bool,
}

pub fn search(
    board: Chess,
    searching: Arc<AtomicBool>,
    debug: Arc<AtomicBool>,
    max_depth: Option<u8>,
    plies_since_irreversible_move: u64,
    mut position_history: &mut Vec<Zobrist64>,
    transposition_table: Arc<Mutex<Vec<Option<Node>>>>,
) {
    let mut actively_searched_node: Result<Node, &'static str>;
    let mut fully_searched_node: Option<Node> = None;

    let start_time = SystemTime::now();

    let mut transposition_table = transposition_table.lock().unwrap();

    let mut principal_variation: Vec<Move>;

    for depth in 1..u8::MAX {
        if let Some(max_depth) = max_depth {
            if depth > max_depth {
                break;
            }
        }

        if !searching.load(Ordering::Relaxed) {
            break;
        }

        let mut alpha = -10000;
        let mut beta = 10000;

        let mut searched_nodes: u64 = 0;

        let hash = board.zobrist_hash::<Zobrist64>(EnPassantMode::Legal).0;

        actively_searched_node = negamax(
            &board,
            depth,
            &mut alpha,
            &mut beta,
            if board.turn() == Color::White { 1 } else { -1 },
            &searching,
            &mut searched_nodes,
            plies_since_irreversible_move,
            &mut position_history,
            &mut transposition_table,
            hash,
        );

        if let Ok(node) = actively_searched_node {
            fully_searched_node = Some(node.clone());

            principal_variation =
                get_principal_variation(&mut board.clone(), depth, &transposition_table);

            if debug.load(Ordering::Relaxed) {
                print_info(
                    &node,
                    start_time,
                    searched_nodes,
                    depth,
                    &principal_variation,
                );
            }

            // If all of the moves lead into terminal nodes, stop searching
            if node.terminal {
                break;
            }
        } else {
            break;
        }
    }

    if let Some(fully_searched_node) = fully_searched_node {
        if let Some(best_move) = fully_searched_node.best_move {
            println!(
                "bestmove {}",
                best_move.to_uci(CastlingMode::Standard).to_string()
            )
        }
    }

    searching.store(false, Ordering::Relaxed);
}

fn print_info(
    node: &Node,
    start_time: SystemTime,
    searched_nodes: u64,
    depth: u8,
    principal_variation: &Vec<Move>,
) {
    let time_ms = start_time.elapsed().unwrap().as_millis();
    let nodes_per_second: u64 = searched_nodes / (time_ms + 1) as u64 * 1000;
    let score: String;

    let pv_string: String = principal_variation
        .iter()
        .map(|m| m.to_uci(CastlingMode::Standard).to_string())
        .collect::<Vec<String>>()
        .join(" ");

    if let Some(mate_in_plies) = node.mate_in_plies {
        score = format!(
            "mate {}",
            ((mate_in_plies as f64 / 2.0).ceil() as i8).to_string()
        );
    } else {
        score = format!("cp {}", node.score);
    }

    println!("info depth {depth} score {score} time {time_ms} nodes {searched_nodes} nps {nodes_per_second} pv {pv_string}");
}

fn negamax(
    board: &Chess,
    depth: u8,
    alpha: &mut i16,
    beta: &mut i16,
    color: i16,
    searching: &Arc<AtomicBool>,
    nodes: &mut u64,
    plies_since_irreversible_move: u64,
    mut position_history: &mut Vec<Zobrist64>,
    transposition_table: &mut Vec<Option<Node>>,
    hash: u64,
) -> Result<Node, &'static str> {
    if !searching.load(Ordering::Relaxed) {
        return Err("Incomplete search");
    }

    let mut node: Node = Node {
        hash: hash,
        score: 0,
        best_move: None,
        depth: depth,
        node_type: NodeType::UPPERBOUND,
        mate_in_plies: None,
        terminal: true,
    };

    let transposition_table_index: usize = hash as usize % TRANSPOSITION_TABLE_LENGTH;

    if board.is_insufficient_material() {
        return Ok(node);
    }

    let mut legal_moves: MoveList = board.legal_moves();

    // Checkmate or stalemate
    if legal_moves.is_empty() {
        if !board.checkers().is_empty() {
            node.score = -20000;
            node.mate_in_plies = Some(0);
        }

        return Ok(node);
    }
    // 50-move rule
    else {
        if board.halfmoves() >= 100 {
            return Ok(node);
        }
    }

    // Threefold repetition
    if plies_since_irreversible_move >= 8 {
        let current_board_hash: Zobrist64 = board.zobrist_hash(EnPassantMode::Legal);

        let mut repetitions: u64 = 0;

        for position in &mut *position_history {
            if *position == current_board_hash {
                repetitions += 1;

                if repetitions > 2 {
                    return Ok(node);
                }
            }
        }
    }

    // Transposition table hit
    if let Some(ref tt_node) = transposition_table[transposition_table_index] {
        if tt_node.hash == hash && tt_node.depth >= depth {
            node = tt_node.clone();

            if node.node_type == NodeType::EXACT {
                return Ok(node);
            } else if node.node_type == NodeType::LOWERBOUND {
                *alpha = i16::max(*alpha, node.score);
            } else if node.node_type == NodeType::UPPERBOUND {
                *beta = i16::min(*beta, node.score);
            }
            if alpha >= beta {
                return Ok(node);
            }
        }
    }

    if depth == 0 {
        node.score = color * evaluate::evaluate(&board);
        node.terminal = false;

        return Ok(node);
    }

    node.score = -30000;

    sort_legal_moves(&mut legal_moves, board, hash, transposition_table);

    for legal_move in legal_moves {
        *nodes += 1;

        let mut board_clone = board.clone();

        board_clone.play_unchecked(&legal_move);

        let child_hash = board_clone.zobrist_hash(EnPassantMode::Legal);

        position_history.push(child_hash);

        let plies_since_irreversible_move = if board.is_irreversible(&legal_move) {
            0
        } else {
            plies_since_irreversible_move + 1
        };

        let child_node = negamax(
            &board_clone,
            depth - 1,
            &mut -(*beta),
            &mut -(*alpha),
            -color,
            &searching,
            nodes,
            plies_since_irreversible_move,
            &mut position_history,
            transposition_table,
            child_hash.0,
        )?;

        position_history.pop();

        if !child_node.terminal {
            node.terminal = false;
        }

        if -child_node.score > node.score {
            node.score = -child_node.score;
            node.best_move = Some(legal_move);

            if node.score > *alpha {
                *alpha = node.score;

                node.node_type = NodeType::EXACT;
            }

            if let Some(child_mate_in_plies) = child_node.mate_in_plies {
                if child_mate_in_plies == 0 {
                    // We have mate in one
                    node.mate_in_plies = Some(1);
                } else if child_mate_in_plies > 0 {
                    // Opponent has mate in x plies
                    node.mate_in_plies = Some(-child_mate_in_plies - 1);
                } else if child_mate_in_plies < 0 {
                    // We have mate in x plies
                    node.mate_in_plies = Some(-child_mate_in_plies + 1);
                }

                node.terminal = true;
            } else {
                node.mate_in_plies = None;
            }
        }

        if node.score >= *beta {
            node.node_type = NodeType::LOWERBOUND;

            break;
        }
    }

    // Store node in the transposition table
    if searching.load(Ordering::Relaxed) {
        if let Some(ref tt_node) = transposition_table[transposition_table_index] {
            if tt_node.depth < depth {
                transposition_table[transposition_table_index] = Some(node.clone());
            }
        } else {
            transposition_table[transposition_table_index] = Some(node.clone());
        }
    }

    return Ok(node);
}

fn sort_legal_moves(
    legal_moves: &mut MoveList,
    board: &Chess,
    hash: u64,
    transposition_table: &Vec<Option<Node>>,
) {
    if legal_moves.len() == 0 {
        return;
    }

    // Move best move to the front
    if let Some(ref pv_node) = transposition_table[hash as usize % TRANSPOSITION_TABLE_LENGTH] {
        if let Some(ref best_move) = pv_node.best_move {
            if board.is_legal(best_move) {
                if let Some(pos) = legal_moves.iter().position(|m| m == best_move) {
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

fn get_principal_variation(
    board: &mut Chess,
    depth: u8,
    transposition_table: &Vec<Option<Node>>,
) -> Vec<Move> {
    let mut pv: Vec<Move> = Vec::new();

    let mut hash: u64;

    for _ in 0..depth {
        hash = board.zobrist_hash::<Zobrist64>(EnPassantMode::Legal).0;

        if let Some(ref pv_node) = transposition_table[hash as usize % TRANSPOSITION_TABLE_LENGTH] {
            if let Some(ref best_move) = pv_node.best_move {
                if board.is_legal(best_move) {
                    pv.push(best_move.clone());

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

    return pv;
}

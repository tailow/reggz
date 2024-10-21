use crate::engine::TRANSPOSITION_TABLE_LENGTH;
use crate::evaluate;
use shakmaty::zobrist::{Zobrist64, ZobristHash};
use shakmaty::{CastlingMode, Chess, Color, EnPassantMode, Move, Position};
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
    position_history: Vec<Zobrist64>,
    transposition_table: Arc<Mutex<Vec<Option<Node>>>>,
) {
    let mut actively_searched_node: Result<Node, &'static str>;
    let mut fully_searched_node: Result<Node, &'static str> = Err("Incomplete search");

    let start_time = SystemTime::now();

    let mut transposition_table = transposition_table.lock().unwrap();

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
            &position_history,
            &mut transposition_table,
            hash,
        );

        if !searching.load(Ordering::Relaxed) {
            break;
        }

        fully_searched_node = actively_searched_node;

        if debug.load(Ordering::Relaxed) && fully_searched_node.is_ok() {
            print_info(
                fully_searched_node.clone().unwrap(),
                start_time,
                searched_nodes,
                depth,
            );
        }

        // If all of the moves lead into terminal nodes, stop searching
        if let Ok(node) = fully_searched_node.clone() {
            if node.terminal {
                break;
            }
        }
    }

    if let Ok(fully_searched_node) = fully_searched_node {
        if let Some(best_move) = fully_searched_node.best_move {
            println!(
                "bestmove {}",
                best_move.to_uci(CastlingMode::Standard).to_string()
            )
        }
    }

    searching.store(false, Ordering::Relaxed);
}

fn print_info(node: Node, start_time: SystemTime, searched_nodes: u64, depth: u8) {
    let time_ms = start_time.elapsed().unwrap().as_millis();
    let nodes_per_second: u64 = searched_nodes / (time_ms + 1) as u64 * 1000;
    let score: String;

    let mut best_move: String = String::from("none");

    if let Some(bmove) = node.best_move {
        best_move = bmove.to_uci(CastlingMode::Standard).to_string();
    }

    if let Some(mate_in_plies) = node.mate_in_plies {
        score = format!(
            "mate {}",
            ((mate_in_plies as f64 / 2.0).ceil() as i8).to_string()
        );
    } else {
        score = format!("cp {}", node.score);
    }

    println!("info depth {depth} score {score} time {time_ms} nodes {searched_nodes} nps {nodes_per_second} pv {best_move}");
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
    position_history: &Vec<Zobrist64>,
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

    let legal_moves = board.legal_moves();

    // Checkmate or stalemate
    if legal_moves.is_empty() {
        if !board.checkers().is_empty() {
            node.score = -1000;
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

        for position in position_history {
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

    node.score = i16::MIN;

    for legal_move in legal_moves {
        *nodes += 1;

        let mut board_clone = board.clone();

        board_clone.play_unchecked(&legal_move);

        let child_hash = board_clone
            .zobrist_hash::<Zobrist64>(EnPassantMode::Legal)
            .0;

        let mut position_history_clone = position_history.clone();

        position_history_clone.push(board_clone.zobrist_hash(EnPassantMode::Legal));

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
            &position_history_clone,
            transposition_table,
            child_hash,
        )?;

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

    if searching.load(Ordering::Relaxed) {
        if let Some(ref tt_node) = transposition_table[transposition_table_index] {
            if tt_node.depth < depth {
                if node.score <= *alpha {
                    node.node_type = NodeType::UPPERBOUND;
                }

                transposition_table[transposition_table_index] = Some(node.clone());
            }
        } else {
            transposition_table[transposition_table_index] = Some(node.clone());
        }
    }

    return Ok(node);
}

use crate::evaluate;
use shakmaty::{CastlingMode, Chess, Color, Move, MoveList, Position};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time::Duration};

#[derive(Debug, Clone)]
struct Node {
    pub score: f32,
    pub best_move: Option<Move>,
}

pub fn search(
    board: Chess,
    searching: Arc<AtomicBool>,
    pondering: Arc<AtomicBool>,
    debug: Arc<AtomicBool>,
    max_depth: Option<u32>,
) {
    let mut depth: u32 = 1;

    let mut actively_searched_node: Option<Node>;
    let mut fully_searched_node: Option<Node> = None;

    loop {
        if let Some(max_depth) = max_depth {
            if depth > max_depth {
                break;
            }
        }

        if !searching.load(Ordering::Relaxed) {
            break;
        }

        let mut alpha = -f32::INFINITY;
        let mut beta = f32::INFINITY;

        if board.turn() == Color::White {
            actively_searched_node = negamax(&board, depth, &mut alpha, &mut beta, 1.0, &searching);
        } else {
            actively_searched_node =
                negamax(&board, depth, &mut alpha, &mut beta, -1.0, &searching);
        }

        if searching.load(Ordering::Relaxed) {
            fully_searched_node = actively_searched_node;

            if debug.load(Ordering::Relaxed) {
                println!("info depth {depth}")
            }
        }

        depth += 1;
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

fn negamax(
    board: &Chess,
    depth: u32,
    alpha: &mut f32,
    beta: &mut f32,
    color: f32,
    searching: &Arc<AtomicBool>,
) -> Option<Node> {
    if !searching.load(Ordering::Relaxed) {
        return None;
    }

    let mut node: Node = Node {
        score: 0.0,
        best_move: None,
    };

    if board.is_checkmate() {
        if board.turn() == Color::White {
            node.score = f32::NEG_INFINITY;
        } else {
            node.score = f32::INFINITY;
        }

        return Some(node);
    }

    if board.is_stalemate() || board.is_insufficient_material() {
        return Some(node);
    }

    if depth == 0 {
        node.score = color * evaluate::evaluate(&board);

        return Some(node);
    }

    let legal_moves: MoveList = board.legal_moves();

    for legal_move in legal_moves {
        let mut board_clone = board.clone();

        board_clone.play_unchecked(&legal_move);

        let child_node = negamax(
            &board_clone,
            depth - 1,
            &mut -(*beta),
            &mut -(*alpha),
            -color,
            &searching,
        );

        match child_node {
            Some(child_node) => {
                if -child_node.score >= node.score {
                    node.score = -child_node.score;
                    node.best_move = Some(legal_move);
                }

                *alpha = f32::max(*alpha, node.score);

                if *alpha >= *beta {
                    break;
                }
            }
            None => return None,
        }
    }

    return Some(node);
}

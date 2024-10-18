use crate::evaluate;
use shakmaty::{CastlingMode, Chess, Color, Move, Position};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

#[derive(Debug, Clone)]
struct Node {
    pub score: f32,
    pub best_move: Option<Move>,
    pub mate_in_plies: Option<i8>,
}

pub fn search(
    board: Chess,
    searching: Arc<AtomicBool>,
    pondering: Arc<AtomicBool>,
    debug: Arc<AtomicBool>,
    max_depth: Option<u64>,
) {
    let mut depth: u64 = 1;

    let mut actively_searched_node: Result<Node, &'static str>;
    let mut fully_searched_node: Result<Node, &'static str> = Err("Incomplete search");

    let start_time = SystemTime::now();

    let mut searched_nodes: u64 = 0;

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
            actively_searched_node = negamax(
                &board,
                depth,
                &mut alpha,
                &mut beta,
                1.0,
                &searching,
                &mut searched_nodes,
            );
        } else {
            actively_searched_node = negamax(
                &board,
                depth,
                &mut alpha,
                &mut beta,
                -1.0,
                &searching,
                &mut searched_nodes,
            );
        }

        if searching.load(Ordering::Relaxed) {
            fully_searched_node = actively_searched_node;

            if debug.load(Ordering::Relaxed) {
                let time_ms = start_time.elapsed().unwrap().as_millis();
                let nodes_per_second: u64 = searched_nodes / (time_ms + 1) as u64 * 1000;
                let score: String;

                let node: Node = fully_searched_node.clone().ok().unwrap();

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
                    score = format!("cp {}", (node.score * 100.0) as i64);
                }

                println!(
                    "info depth {depth} score {score} time {time_ms} nodes {searched_nodes} nps {nodes_per_second} string pv {best_move}"
                )
            }
        }

        depth += 1;
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

fn negamax(
    board: &Chess,
    depth: u64,
    alpha: &mut f32,
    beta: &mut f32,
    color: f32,
    searching: &Arc<AtomicBool>,
    nodes: &mut u64,
) -> Result<Node, &'static str> {
    if !searching.load(Ordering::Relaxed) {
        return Err("Incomplete search");
    }

    *nodes += 1;

    let mut node: Node = Node {
        score: 0.0,
        best_move: None,
        mate_in_plies: None,
    };

    if board.is_checkmate() {
        node.score = -1000.0;
        node.mate_in_plies = Some(0);

        return Ok(node);
    }

    if board.is_stalemate() || board.is_insufficient_material() {
        return Ok(node);
    }

    if depth == 0 {
        node.score = color * evaluate::evaluate(&board);

        return Ok(node);
    }

    node.score = f32::NEG_INFINITY;

    for legal_move in board.legal_moves() {
        let mut board_clone = board.clone();

        board_clone.play_unchecked(&legal_move);

        let child_node = negamax(
            &board_clone,
            depth - 1,
            &mut -(*beta),
            &mut -(*alpha),
            -color,
            &searching,
            nodes,
        );

        match child_node {
            Ok(child_node) => {
                if -child_node.score > node.score {
                    node.score = -child_node.score;
                    node.best_move = Some(legal_move);

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
                    } else {
                        node.mate_in_plies = None;
                    }
                }

                *alpha = f32::max(*alpha, node.score);

                if *alpha >= *beta {
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }

    return Ok(node);
}

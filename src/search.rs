use shakmaty::{CastlingMode, Chess, Color, Move, Position};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time::Duration};

struct Node {
    pub score: f32,
    pub best_move: Move,
    pub depth: u8,
    pub mate: u8,
}

pub fn search(
    board: Chess,
    searching: Arc<AtomicBool>,
    pondering: Arc<AtomicBool>,
    debug: Arc<AtomicBool>,
    max_depth: Option<u32>,
) {
    let depth: u32 = 1;

    let mut current_node: Option<Node> = None;
    let mut previous_node: Option<Node> = None;

    loop {
        if let Some(max_depth) = max_depth {
            if depth > max_depth {
                break;
            }
        }

        if searching.load(Ordering::Relaxed) == true {
            if board.turn() == Color::White {
                current_node = negamax(&board, depth, -f32::INFINITY, f32::INFINITY, 1);
            } else {
                current_node = negamax(&board, depth, -f32::INFINITY, f32::INFINITY, -1);
            }

            if searching.load(Ordering::Relaxed) == true {
                previous_node = current_node;

                if debug.load(Ordering::Relaxed) == true {
                    println!("info depth {depth}")
                }
            }
        }
    }

    if let Some(previous_node) = previous_node {
        println!(
            "bestmove {}",
            previous_node
                .best_move
                .to_uci(CastlingMode::Standard)
                .to_string()
        )
    }

    searching.store(false, Ordering::Relaxed);
}

fn negamax(board: &Chess, depth: u32, alpha: f32, beta: f32, color: i8) -> Option<Node> {
    let node: Option<Node> = None;

    node
}
